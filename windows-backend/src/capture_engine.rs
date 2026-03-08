#![cfg(target_os = "windows")]

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

pub struct CaptureEngine {
    d3d_device: IDirect3DDevice,
    device: ID3D11Device,
    context: ID3D11DeviceContext,
}

impl CaptureEngine {
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

    pub fn start<F>(
        &mut self,
        hwnd: HWND,
        running: Arc<AtomicBool>,
        mut on_frame: F,
    ) -> Result<()>
    where
        F: FnMut(u32, u32, Vec<u8>) + Send + 'static,
    {
        let item = create_capture_item(hwnd)?;
        let size = item.Size()?;

        let frame_pool = Direct3D11CaptureFramePool::Create(
            &self.d3d_device,
            DirectXPixelFormat::B8G8R8A8UIntNormalized,
            2,
            size,
        )?;

        let session = frame_pool.CreateCaptureSession(&item)?;
        session.StartCapture()?;

        let device = self.device.clone();
        let context = self.context.clone();
        let running_cb = running.clone();

        let mut staging_tex: Option<ID3D11Texture2D> = None;
        let mut width = 0;
        let mut height = 0;

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

                        context.Map(
                            staging,
                            0,
                            D3D11_MAP_READ,
                            0,
                            Some(&mut mapped),
                        )?;

                        let row_pitch = mapped.RowPitch as usize;

                        let mut data = vec![0u8; (width * height * 4) as usize];

                        let src = mapped.pData as *const u8;

                        for y in 0..height as usize {
                            let src_row = src.add(y * row_pitch);
                            let dst_row =
                                data.as_mut_ptr().add(y * width as usize * 4);

                            std::ptr::copy_nonoverlapping(
                                src_row,
                                dst_row,
                                width as usize * 4,
                            );
                        }

                        context.Unmap(staging, 0);

                        // BGRA -> RGBA
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

                        on_frame(width, height, data);
                    }

                    Ok(())
                },
            ),
        )?;

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

fn create_capture_item(hwnd: HWND) -> Result<GraphicsCaptureItem> {
    unsafe {
        let interop: IGraphicsCaptureItemInterop =
            windows::core::factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>()?;

        interop.CreateForWindow(hwnd)
    }
}

fn get_texture(surface: &IDirect3DSurface) -> Result<ID3D11Texture2D> {
    unsafe {
        let access: IDirect3DDxgiInterfaceAccess = surface.cast()?;
        access.GetInterface()
    }
}