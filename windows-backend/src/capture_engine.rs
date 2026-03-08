// Windows Graphics Capture (WGC) - это API Windows 10+,
// для захвата отдельного окна, всего экрана, региона
// Использует: D3D11, WinRT, GPU textures, Frame pool
#![cfg(target_os = "windows")]
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
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
        Graphics::{
            Direct3D::*,
            Direct3D11::*,
            Dxgi::*,
        },
        System::WinRT::{
            Direct3D11::{
                CreateDirect3D11DeviceFromDXGIDevice,
                IDirect3DDxgiInterfaceAccess,
            },
            Graphics::Capture::IGraphicsCaptureItemInterop,
            *,
        },
        UI::WindowsAndMessaging::*,
    },
};
use windows::core::IInspectable;

pub struct CaptureEngine {
    d3d_device: IDirect3DDevice,
    running: Arc<AtomicBool>,
}

impl CaptureEngine {
    pub fn init() -> Result<Self> {
        // Шаг 1 — Инициализация COM
        // unsafe { RoInitialize(RO_INIT_MULTITHREADED)?; }
        // может работать лучше
        unsafe { RoInitialize(RO_INIT_SINGLETHREADED)?; }

        // Шаг 2 — Создание D3D11 устройства
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
        let dxgi_device: IDXGIDevice = device.cast()?;
        let inspectable: IInspectable = unsafe { CreateDirect3D11DeviceFromDXGIDevice(&dxgi_device)? };
        let d3d_device: IDirect3DDevice = inspectable.cast()?;

        Ok(Self {
            d3d_device,
            running: Arc::new(AtomicBool::new(false)),
        })
    }

    pub fn stop(&self) -> Result<()> {
        self.running.store(false, Ordering::SeqCst);
        Ok(())
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
        self.stop()?;
        // Шаг 4 — Создать FramePool
        let item = create_capture_item(hwnd)?;
        let size = item.Size()?;

        let frame_pool = Direct3D11CaptureFramePool::Create(
            &self.d3d_device,
            DirectXPixelFormat::B8G8R8A8UIntNormalized,
            2,
            size,
        )?;

        // Шаг 5 — Сессия
        let session = frame_pool.CreateCaptureSession(&item)?;
        session.StartCapture()?;

        // Шаг 6 — Получение кадров
        let _token = frame_pool.FrameArrived(
            &TypedEventHandler::<Direct3D11CaptureFramePool, IInspectable>::new(
                move |pool: &Option<Direct3D11CaptureFramePool>, _| {
                    if let Some(pool) = pool {
                        if let Ok(frame) = pool.TryGetNextFrame() {

                            if let Ok(surface) = frame.Surface() {

                                match get_texture_from_surface(&surface) {

                                    Ok(texture) => {

                                        let mut desc = D3D11_TEXTURE2D_DESC::default();

                                        unsafe {
                                            texture.GetDesc(&mut desc);
                                        }

                                        println!(
                                            "TEXTURE: {}x{}  format={:?}  mip_levels={}  usage={:?}",
                                            desc.Width,
                                            desc.Height,
                                            desc.Format,
                                            desc.MipLevels,
                                            desc.Usage
                                        );

                                    }

                                    Err(e) => {
                                        eprintln!("Texture cast failed: {:?}", e);
                                    }
                                }
                            }
                        }
                    }
                    Ok(())
                },
            ),
        )?;

        // Message pump для MTA WinRT
        while running.load(Ordering::SeqCst) {
            unsafe {
                let mut msg = std::mem::MaybeUninit::<MSG>::uninit();
                let pmsg = msg.as_mut_ptr();

                if PeekMessageW(pmsg, HWND(std::ptr::null_mut()), 0, 0, PM_REMOVE).as_bool() {
                    let msg = msg.assume_init();
                    #[allow(unused)]
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }

            std::thread::sleep(std::time::Duration::from_millis(5));
        }

        session.Close()?;
        frame_pool.Close()?;

        Ok(())
    }
}

// Шаг 3 — Получить GraphicsCaptureItem из HWND
// тут окно изолируется от перекрытий
fn create_capture_item(hwnd: HWND) -> Result<GraphicsCaptureItem> {
    unsafe {
        let interop: IGraphicsCaptureItemInterop =
            windows::core::factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>()?;
        interop.CreateForWindow(hwnd)
    }
}

fn get_texture_from_surface(surface: &IDirect3DSurface) -> Result<ID3D11Texture2D> {
    unsafe {
        let access: IDirect3DDxgiInterfaceAccess = surface.cast()?;
        let texture: ID3D11Texture2D = access.GetInterface()?;
        Ok(texture)
    }
}
