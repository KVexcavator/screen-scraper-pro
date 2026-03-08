#![cfg(target_os = "windows")]

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
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
            *,
        },
        UI::WindowsAndMessaging::*,
    },
};

pub struct CaptureEngine {
    d3d_device: IDirect3DDevice,
    device: ID3D11Device,
    context: ID3D11DeviceContext,
    inner: Arc<Mutex<CaptureEngineInner>>,
    running: Arc<AtomicBool>,
}

struct CaptureEngineInner {
    staging: Option<ID3D11Texture2D>,
    width: u32,
    height: u32,
}

impl CaptureEngine {
    pub fn init() -> Result<Self> {
        unsafe { RoInitialize(RO_INIT_SINGLETHREADED)?; }

        let mut device: Option<ID3D11Device> = None;
        let mut context: Option<ID3D11DeviceContext> = None;

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
        let inspectable: IInspectable =
            unsafe { CreateDirect3D11DeviceFromDXGIDevice(&dxgi_device)? };
        let d3d_device: IDirect3DDevice = inspectable.cast()?;

        Ok(Self {
            d3d_device,
            device,
            context,
            inner: Arc::new(Mutex::new(CaptureEngineInner {
                staging: None,
                width: 0,
                height: 0,
            })),
            running: Arc::new(AtomicBool::new(false)),
        })
    }

    pub fn stop(&self) -> Result<()> {
        self.running.store(false, Ordering::SeqCst);
        Ok(())
    }

    fn ensure_staging(&self, desc: &D3D11_TEXTURE2D_DESC) -> Result<ID3D11Texture2D> {
        let mut inner = self.inner.lock().unwrap();
        if let Some(staging) = &inner.staging {
            return Ok(staging.clone());
        }

        let mut staging_desc = *desc;
        staging_desc.BindFlags = 0;
        staging_desc.CPUAccessFlags = D3D11_CPU_ACCESS_READ.0 as u32;
        staging_desc.Usage = D3D11_USAGE_STAGING;
        staging_desc.MiscFlags = 0;

        let mut staging = None;
        unsafe { self.device.CreateTexture2D(&staging_desc, None, Some(&mut staging))?; }

        let staging = staging.unwrap();
        inner.width = desc.Width;
        inner.height = desc.Height;
        inner.staging = Some(staging.clone());

        Ok(staging)
    }

    fn copy_to_cpu(&self, texture: &ID3D11Texture2D) -> Result<Vec<u8>> {
        let staging = {
            let mut desc = D3D11_TEXTURE2D_DESC::default();
            unsafe { texture.GetDesc(&mut desc); }
            self.ensure_staging(&desc)?
        };

        unsafe {
            self.context.CopyResource(&staging, texture);

            let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
            self.context.Map(&staging, 0, D3D11_MAP_READ, 0, Some(&mut mapped))?;

            let inner = self.inner.lock().unwrap();
            let width = inner.width;
            let height = inner.height;

            let row_pitch = mapped.RowPitch as usize;
            let mut data = vec![0u8; (width * height * 4) as usize];

            let src = mapped.pData as *const u8;
            for y in 0..height as usize {
                let src_row = src.add(y * row_pitch);
                let dst_row = data.as_mut_ptr().add(y * width as usize * 4);
                std::ptr::copy_nonoverlapping(src_row, dst_row, width as usize * 4);
            }

            // BGRX -> RGBA
            for i in 0..(width * height) as usize {
                let b = data[i * 4 + 0];
                let g = data[i * 4 + 1];
                let r = data[i * 4 + 2];
                let a = data[i * 4 + 3];

                data[i * 4 + 0] = r;
                data[i * 4 + 1] = g;
                data[i * 4 + 2] = b;
                data[i * 4 + 3] = a;
            }

            self.context.Unmap(&staging, 0);
            Ok(data)
        }
    }

    pub fn start<F>(&self, hwnd: HWND, running: Arc<AtomicBool>, mut on_frame: F) -> Result<()>
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

        let engine_inner = self.inner.clone();
        let device = self.device.clone();
        let context = self.context.clone();

        let _token = frame_pool.FrameArrived(
            &TypedEventHandler::<Direct3D11CaptureFramePool, IInspectable>::new(
                move |pool, _| {
                    if let Some(pool) = pool {
                        if let Ok(frame) = pool.TryGetNextFrame() {
                            if let Ok(surface) = frame.Surface() {
                                if let Ok(texture) = get_raw_textures(&surface) {
                                    // Используем существующий engine_inner, device и context
                                    let pixels = {
                                        let mut desc = D3D11_TEXTURE2D_DESC::default();
                                        unsafe { texture.GetDesc(&mut desc); }

                                        let staging = {
                                            let mut inner = engine_inner.lock().unwrap();

                                            if inner.staging.is_none() {
                                                let mut staging_desc = desc;
                                                staging_desc.BindFlags = 0;
                                                staging_desc.CPUAccessFlags = D3D11_CPU_ACCESS_READ.0 as u32;
                                                staging_desc.Usage = D3D11_USAGE_STAGING;
                                                staging_desc.MiscFlags = 0;

                                                let mut tex = None;
                                                unsafe { device.CreateTexture2D(&staging_desc, None, Some(&mut tex)).unwrap() };

                                                inner.width = desc.Width;
                                                inner.height = desc.Height;
                                                inner.staging = tex;
                                            }

                                            inner.staging.clone().unwrap()
                                        };

                                        unsafe {
                                            context.CopyResource(&staging, &texture);

                                            let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
                                            context.Map(&staging, 0, D3D11_MAP_READ, 0, Some(&mut mapped)).unwrap();

                                            let inner = engine_inner.lock().unwrap();
                                            let w = inner.width;
                                            let h = inner.height;

                                            let row_pitch = mapped.RowPitch as usize;
                                            let mut data = vec![0u8; (w * h * 4) as usize];

                                            let src = mapped.pData as *const u8;

                                            for y in 0..h as usize {
                                                let src_row = src.add(y * row_pitch);
                                                let dst_row = data.as_mut_ptr().add(y * w as usize * 4);
                                                std::ptr::copy_nonoverlapping(src_row, dst_row, w as usize * 4);
                                            }

                                            context.Unmap(&staging, 0);

                                            Ok::<_, windows::core::Error>((w, h, data))
                                        }
                                    };

                                    if let Ok((w, h, data)) = pixels {
                                        on_frame(w, h, data);
                                    }
                                }
                            }
                        }
                    }
                    Ok(())
                },
            ),
        )?;

        while running.load(Ordering::SeqCst) {
            unsafe {
                let mut msg = std::mem::MaybeUninit::<MSG>::uninit();
                let pmsg = msg.as_mut_ptr();

                if PeekMessageW(pmsg, HWND(std::ptr::null_mut()), 0, 0, PM_REMOVE).as_bool() {
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

fn get_raw_textures(surface: &IDirect3DSurface) -> Result<ID3D11Texture2D> {
    unsafe {
        let access: IDirect3DDxgiInterfaceAccess = surface.cast()?;
        Ok(access.GetInterface()?)
    }
}