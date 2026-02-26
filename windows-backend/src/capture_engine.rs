#![cfg(target_os = "windows")]
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use windows::Foundation::TypedEventHandler;
use windows::Win32::System::WinRT::Direct3D11::CreateDirect3D11DeviceFromDXGIDevice;
use windows::Win32::System::WinRT::Graphics::Capture::IGraphicsCaptureItemInterop;
use windows::core::IInspectable;

use windows::{
    Graphics::Capture::*,
    Graphics::DirectX::Direct3D11::*,
    Graphics::DirectX::*,
    Win32::{
        Foundation::HWND,
        Graphics::{Direct3D::*, Direct3D11::*, Dxgi::*},
        System::WinRT::*,
    },
    core::*,
};

pub struct CaptureEngine {
    d3d_device: IDirect3DDevice,
}

impl CaptureEngine {
    pub fn init() -> Result<Self> {
        unsafe {
            RoInitialize(RO_INIT_MULTITHREADED)?;
        }

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
        let inspectable: IInspectable =
            unsafe { CreateDirect3D11DeviceFromDXGIDevice(&dxgi_device)? };
        let d3d_device: IDirect3DDevice = inspectable.cast()?;

        Ok(Self { d3d_device })
    }

    pub fn start<F>(&self, hwnd: HWND, mut on_frame: F) -> Result<()>
    where
        F: FnMut() + Send + 'static,
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

        let fps_counter = Arc::new(Mutex::new((0u32, Instant::now())));

        frame_pool.FrameArrived(
            &TypedEventHandler::<Direct3D11CaptureFramePool, IInspectable>::new({
                let fps_counter = fps_counter.clone();

                move |pool: &Option<Direct3D11CaptureFramePool>, _| {
                    if let Some(pool) = pool
                        && let Ok(_frame) = pool.TryGetNextFrame()
                    {
                        let mut data = fps_counter.lock().unwrap();
                        data.0 += 1;

                        if data.1.elapsed() >= Duration::from_secs(1) {
                            println!("FPS: {}", data.0);
                            data.0 = 0;
                            data.1 = Instant::now();
                        }

                        on_frame();
                    }
                    Ok(())
                }
            }),
        )?;

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
