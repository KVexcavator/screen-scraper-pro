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
                                match get_raw_textures(&surface) {
                                    Ok(texture) => {
                                        match get_readable_textures(&texture) {

                                            Ok((w, h, pixels)) => {

                                                println!(
                                                    "FRAME CPU {}x{} bytes={}",
                                                    w,
                                                    h,
                                                    pixels.len()
                                                );

                                                on_frame(w, h, pixels);
                                            }
                                            Err(e) => { eprintln!("Readable textures failed {:?}", e); }
                                        }
                                    }
                                    Err(e) => { eprintln!("Raw textures failed: {:?}", e); }
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

fn get_raw_textures(surface: &IDirect3DSurface) -> Result<ID3D11Texture2D> {
    unsafe {
        let access: IDirect3DDxgiInterfaceAccess = surface.cast()?;
        let texture: ID3D11Texture2D = access.GetInterface()?;
        Ok(texture)
    }
}

fn get_readable_textures(
    texture: &ID3D11Texture2D,
) -> Result<(u32, u32, Vec<u8>)> {

    unsafe {
        // get device
        let device: ID3D11Device = texture.GetDevice()?;
        // get context
        let context: ID3D11DeviceContext = device.GetImmediateContext()?;
        // get description of texture
        let mut desc = D3D11_TEXTURE2D_DESC::default();
        texture.GetDesc(&mut desc);

        println!("SOURCE texture {:?}", desc);

        // create staging descriptor
        let mut staging_desc = desc;
        staging_desc.BindFlags = 0;
        staging_desc.CPUAccessFlags = D3D11_CPU_ACCESS_READ.0 as u32;
        staging_desc.Usage = D3D11_USAGE_STAGING;
        staging_desc.MiscFlags = 0;

        // staging texture
        let mut staging: Option<ID3D11Texture2D> = None;

        device.CreateTexture2D(
            &staging_desc,
            None,
            Some(&mut staging)
        )?;

        let staging = staging.unwrap();

        // copy GPU -> CPU texture
        context.CopyResource(&staging, texture);

        // Map
        let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();

        context.Map(
            &staging,
            0,
            D3D11_MAP_READ,
            0,
            Some(&mut mapped)
        )?;

        let width = desc.Width;
        let height = desc.Height;

        let row_pitch = mapped.RowPitch as usize;

        println!(
            "Mapped memory row_pitch={} expected_row={}",
            row_pitch,
            width as usize * 4
        );

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

        context.Unmap(&staging, 0);

        Ok((width, height, data))
    }
}
