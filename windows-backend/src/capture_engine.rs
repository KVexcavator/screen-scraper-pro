#![cfg(target_os = "windows")]

/*
    Windows Graphics Capture Engine

    This module implements window capture using the Windows Graphics Capture API.

    Pipeline overview:

        Window (HWND)
              │
              ▼
        GraphicsCaptureItem
              │
              ▼
        Direct3D11CaptureFramePool
              │
              ▼
        GPU Texture (ID3D11Texture2D)
              │
              ▼
        STAGING Texture (CPU readable)
              │
              ▼
        Vec<u8> BGRA
              │
              ▼
        Frame converter
              │
              ▼
        Slint UI preview

    Key characteristics:

    - Uses GPU accelerated capture (WGC)
    - Copies GPU texture → CPU staging texture
    - Converts BGRA → RGBA
    - Sends frames to UI callback
*/

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use windows::{
    core::*,
    Foundation::TypedEventHandler,
    Graphics::{
        Capture::*,
        DirectX::{DirectXPixelFormat, Direct3D11::*},
    },
    Win32::{
        Foundation::HWND,
        Graphics::{Direct3D::*, Direct3D11::*, Dxgi::*},
        System::WinRT::{
            Direct3D11::{CreateDirect3D11DeviceFromDXGIDevice, IDirect3DDxgiInterfaceAccess},
            Graphics::Capture::IGraphicsCaptureItemInterop,
            RoInitialize, RO_INIT_SINGLETHREADED,
        },
        UI::WindowsAndMessaging::*,
    },
};

/// Main capture engine responsible for creating the D3D device
/// and starting the Windows Graphics Capture pipeline.
pub struct CaptureEngine {
    /// WinRT-compatible D3D device used by Windows Graphics Capture
    d3d_device: IDirect3DDevice,

    /// Native Direct3D11 device
    device: ID3D11Device,

    /// Immediate context used for GPU commands
    context: ID3D11DeviceContext,
}

impl CaptureEngine {
    /// Initializes the capture engine.
    ///
    /// Steps:
    ///
    /// 1. Initialize WinRT COM
    /// 2. Create a Direct3D11 device
    /// 3. Convert DXGI device → WinRT IDirect3DDevice
    pub fn init() -> Result<Self> {
        unsafe { RoInitialize(RO_INIT_SINGLETHREADED)? };

        let mut device = None;
        let mut context = None;

        unsafe {
            D3D11CreateDevice(
                None,
                D3D_DRIVER_TYPE_HARDWARE,
                None,
                D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                None,
                D3D11_SDK_VERSION,
                Some(&mut device),
                None,
                Some(&mut context),
            )?;
        }

        let device = device.unwrap();
        let context = context.unwrap();

        let dxgi_device: IDXGIDevice = device.cast()?;

        let inspectable = unsafe { CreateDirect3D11DeviceFromDXGIDevice(&dxgi_device)? };

        let d3d_device: IDirect3DDevice = inspectable.cast()?;

        Ok(Self {
            d3d_device,
            device,
            context,
        })
    }

    /// Starts capturing frames from a specific window.
    ///
    /// `hwnd` – target window handle
    /// `running` – shared flag used to stop the capture loop
    /// `on_frame` – callback delivering RGBA frames to the UI
    pub fn start<F>(
        &mut self,
        hwnd: HWND,
        running: Arc<AtomicBool>,
        mut on_frame: F,
    ) -> Result<()>
    where
        F: FnMut(u32, u32, Vec<u8>) + Send + 'static,
    {
        /*
            STEP 1

            Create capture item from HWND.
            This isolates the window from occlusion and overlays.
        */
        let item = create_capture_item(hwnd)?;

        let size = item.Size()?;

        /*
            STEP 2

            Create frame pool that buffers GPU textures.
        */
        let frame_pool = Direct3D11CaptureFramePool::Create(
            &self.d3d_device,
            DirectXPixelFormat::B8G8R8A8UIntNormalized,
            2,
            size,
        )?;

        /*
            STEP 3

            Create capture session.
        */
        let session = frame_pool.CreateCaptureSession(&item)?;

        session.StartCapture()?;

        let device = self.device.clone();
        let context = self.context.clone();
        let running_cb = running.clone();

        let mut staging_tex: Option<ID3D11Texture2D> = None;

        let mut width = 0;
        let mut height = 0;

        /*
            STEP 4

            Frame callback invoked by WGC
        */
        let _token = frame_pool.FrameArrived(
            &TypedEventHandler::<Direct3D11CaptureFramePool, IInspectable>::new(
                move |pool, _| {
                    if !running_cb.load(Ordering::Relaxed) {
                        return Ok(());
                    }

                    let pool = match pool {
                        Some(p) => p,
                        None => return Ok(()),
                    };

                    let frame = pool.TryGetNextFrame()?;
                    let surface = frame.Surface()?;
                    let texture = get_texture(&surface)?;

                    let mut desc = D3D11_TEXTURE2D_DESC::default();

                    unsafe { texture.GetDesc(&mut desc) };

                    if staging_tex.is_none() {
                        width = desc.Width;
                        height = desc.Height;

                        let mut staging_desc = desc;

                        staging_desc.BindFlags = 0;
                        staging_desc.CPUAccessFlags = D3D11_CPU_ACCESS_READ.0 as u32;
                        staging_desc.Usage = D3D11_USAGE_STAGING;
                        staging_desc.MiscFlags = 0;

                        let mut tex = None;

                        unsafe {
                            device.CreateTexture2D(&staging_desc, None, Some(&mut tex))?;
                        }

                        staging_tex = tex;
                    }

                    let staging = staging_tex.as_ref().unwrap();

                    unsafe {
                        context.CopyResource(staging, &texture);

                        let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();

                        context.Map(staging, 0, D3D11_MAP_READ, 0, Some(&mut mapped))?;

                        let row_pitch = mapped.RowPitch as usize;

                        let mut data = vec![0u8; (width * height * 4) as usize];

                        let src = mapped.pData as *const u8;

                        for y in 0..height as usize {
                            let src_row = src.add(y * row_pitch);

                            let dst_row = data.as_mut_ptr().add(y * width as usize * 4);

                            std::ptr::copy_nonoverlapping(
                                src_row,
                                dst_row,
                                width as usize * 4,
                            );
                        }

                        context.Unmap(staging, 0);

                        convert_bgra_to_rgba(&mut data);

                        on_frame(width, height, data);
                    }

                    Ok(())
                },
            ),
        )?;

        /*
            STEP 5

            WinRT message pump required for capture callbacks
        */
        while running.load(Ordering::Relaxed) {
            unsafe {
                let mut msg = std::mem::MaybeUninit::<MSG>::uninit();

                if PeekMessageW(
                    msg.as_mut_ptr(),
                    HWND(std::ptr::null_mut()),
                    0,
                    0,
                    PM_REMOVE,
                )
                    .as_bool()
                {
                    let msg = msg.assume_init();

                    TranslateMessage(&msg);

                    DispatchMessageW(&msg);
                }
            }

            std::thread::sleep(std::time::Duration::from_millis(2));
        }

        session.Close()?;
        frame_pool.Close()?;

        Ok(())
    }
}

/// Creates GraphicsCaptureItem from window handle.
///
/// This is the entry point for Windows Graphics Capture.
fn create_capture_item(hwnd: HWND) -> Result<GraphicsCaptureItem> {
    unsafe {
        let interop: IGraphicsCaptureItemInterop =
            windows::core::factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>()?;

        interop.CreateForWindow(hwnd)
    }
}

/// Converts WinRT surface → native D3D11 texture.
fn get_texture(surface: &IDirect3DSurface) -> Result<ID3D11Texture2D> {
    unsafe {
        let access: IDirect3DDxgiInterfaceAccess = surface.cast()?;

        access.GetInterface()
    }
}

/// Converts BGRA pixel buffer into RGBA.
///
/// Windows Graphics Capture produces frames in
/// `DXGI_FORMAT_B8G8R8A8_UNORM`.
///
/// Slint expects `RGBA8`.
pub fn convert_bgra_to_rgba(data: &mut [u8]) {
    for px in data.chunks_exact_mut(4) {
        let b = px[0];
        let g = px[1];
        let r = px[2];
        let a = px[3];

        px[0] = r;
        px[1] = g;
        px[2] = b;
        px[3] = a;
    }
}