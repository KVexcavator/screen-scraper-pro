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

pub struct CaptureEngine {
    d3d_device: IDirect3DDevice,
}

impl CaptureEngine {
    pub fn init() -> Result<Self> {
        unsafe { RoInitialize(RO_INIT_MULTITHREADED)?; }

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

        Ok(Self { d3d_device })
    }

    pub fn start<F>(&self, hwnd: HWND, mut on_frame: F) -> Result<()>
    where F: FnMut() + Send + 'static,
    {
        println!("Starting capture for HWND: {:?}", hwnd.0);

        let item = create_capture_item(hwnd)?;
        let size = item.Size()?;
        println!("✓ Capture size: {:?}", size);

        let frame_pool = Direct3D11CaptureFramePool::Create(
            &self.d3d_device,
            DirectXPixelFormat::B8G8R8A8UIntNormalized,
            2,
            size,
        )?;

        let session = frame_pool.CreateCaptureSession(&item)?;
        session.StartCapture()?;
        println!("✓ Session started");

        let _token = frame_pool.FrameArrived(
            &TypedEventHandler::<Direct3D11CaptureFramePool, IInspectable>::new({
                println!("✓ FrameArrived registered");
                move |pool_opt: &Option<Direct3D11CaptureFramePool>, _| {
                    println!("✓ Event fired!");
                    if let Some(pool) = pool_opt {
                        match pool.TryGetNextFrame() {
                            Ok(frame) => {
                                match frame.ContentSize() {
                                    Ok(content_size) => {
                                        println!("🚀 FRAME: {}x{}",
                                                 content_size.Width,
                                                 content_size.Height
                                        );
                                        on_frame();
                                    }
                                    Err(e) => println!("✗ ContentSize error: {:?}", e),
                                }
                            }
                            Err(e) => println!("✗ No frame: {:?}", e),
                        }
                    }
                    Ok(())
                }
            })
        )?;

        // Message pump для MTA WinRT
        loop {
            unsafe {
                let mut msg = std::mem::MaybeUninit::<MSG>::uninit();
                let pmsg = msg.as_mut_ptr();

                if GetMessageW(pmsg, HWND(std::ptr::null_mut()), 0, 0).as_bool() {
                    let msg_filled = msg.assume_init();
                    TranslateMessage(&msg_filled as *const _);
                    DispatchMessageW(&msg_filled as *const _);
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

    }
}

fn create_capture_item(hwnd: HWND) -> Result<GraphicsCaptureItem> {
    println!("Creating capture item for HWND: {:?}", hwnd.0);
    unsafe {
        let interop: IGraphicsCaptureItemInterop =
            windows::core::factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>()?;
        println!("✓ Capture item created");
        interop.CreateForWindow(hwnd)
    }
}
