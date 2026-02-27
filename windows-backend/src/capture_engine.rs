// Windows Graphics Capture (WGC) - это API Windows 10+,
// для захвата отдельного окна, всего экрана, региона
// Использует: D3D11, WinRT, GPU textures, Frame pool
#![cfg(target_os = "windows")]
use windows::Foundation::TypedEventHandler;
use windows::Win32::System::WinRT::Direct3D11::CreateDirect3D11DeviceFromDXGIDevice;
use windows::Win32::System::WinRT::Graphics::Capture::IGraphicsCaptureItemInterop;
use windows::core::IInspectable;
use windows::Graphics::DirectX::DirectXPixelFormat;

use windows::{
    Graphics::Capture::*,
    Graphics::DirectX::Direct3D11::*,
    Win32::{
        Foundation::HWND,
        Graphics::{Direct3D::*, Direct3D11::*, Dxgi::*},
        System::WinRT::*,
        UI::WindowsAndMessaging::*,
    },
    core::*,
};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

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
        eprintln!("Stopping capture engine ==================>>>>>>>>>>>>");
        Ok(())
    }

    pub fn start_with_flag<F>(
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

        let _token = frame_pool.FrameArrived(
            &TypedEventHandler::<Direct3D11CaptureFramePool, IInspectable>::new(
                move |pool: &Option<Direct3D11CaptureFramePool>, _| {
                    if let Some(pool) = pool {
                        if let Ok(frame) = pool.TryGetNextFrame() {
                            if let Ok(size) = frame.ContentSize() {
                                if let (Ok(w), Ok(h)) = (
                                    u32::try_from(size.Width),
                                    u32::try_from(size.Height),
                                ) {
                                    println!("FRAME: {}x{}", w, h);
                                    let fake = vec![255u8; (w * h * 4) as usize];
                                    on_frame(w, h, fake);
                                    // on_frame(w, h, vec![]);
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

    // pub fn start<F>(&self, hwnd: HWND, mut on_frame: F) -> Result<()>
    // where F: FnMut() + Send + 'static,
    // {
    //     self.stop()?;
    //     // Шаг 4 — Создать FramePool
    //     let item = create_capture_item(hwnd)?;
    //     let size = item.Size()?;
    //
    //     let frame_pool = Direct3D11CaptureFramePool::Create(
    //         &self.d3d_device,
    //         DirectXPixelFormat::B8G8R8A8UIntNormalized,
    //         2,
    //         size,
    //     )?;
    //
    //     // Шаг 5 — Сессия
    //     let session = frame_pool.CreateCaptureSession(&item)?;
    //     session.StartCapture()?;
    //
    //     // Шаг 6 — Получение кадров
    //     let _token = frame_pool.FrameArrived(
    //         &TypedEventHandler::<Direct3D11CaptureFramePool, IInspectable>::new({
    //             move |pool_opt: &Option<Direct3D11CaptureFramePool>, _| {
    //                 if let Some(pool) = pool_opt {
    //                     match pool.TryGetNextFrame() {
    //                         Ok(frame) => {
    //                             // TODO create videofile
    //                             // понять что еще есть вo frame
    //                             // например frame.Surface()
    //                             // как получать ID3D11Texture2D
    //                             // дальше либо:
    //                             //  - копировать в staging texture
    //                             //  - отдавать в encoder
    //                             match frame.ContentSize() {
    //                                 Ok(size) => {
    //                                     println!("FRAME: {}x{}", size.Width, size.Height);
    //                                     on_frame();
    //                                 }
    //                                 Err(e) => println!("ContentSize error: {:?}", e),
    //                             }
    //                         }
    //                         Err(e) => println!("No frame: {:?}", e),
    //                     }
    //                 }
    //                 Ok(())
    //             }
    //         })
    //     )?;
    //
    //     // Message pump для MTA WinRT
    //     loop {
    //         unsafe {
    //             let mut msg = std::mem::MaybeUninit::<MSG>::uninit();
    //             let pmsg = msg.as_mut_ptr();
    //
    //             if GetMessageW(pmsg, HWND(std::ptr::null_mut()), 0, 0).as_bool() {
    //                 let msg_filled = msg.assume_init();
    //                 #[allow(unused)]
    //                 TranslateMessage(&msg_filled as *const _);
    //                 DispatchMessageW(&msg_filled as *const _);
    //             }
    //         }
    //         std::thread::sleep(std::time::Duration::from_millis(10));
    //     }
    //
    // }
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
