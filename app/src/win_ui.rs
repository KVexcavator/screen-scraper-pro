use screen_ui::UiHandle;
use screen_ui::*;
use slint::{Image, Rgba8Pixel, SharedPixelBuffer, SharedString};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use windows_backend::capture_engine::CaptureEngine;
use windows_backend::catcher::{WindowInfo, get_windows};
use windows::Win32::Foundation::HWND;

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let ui = UiHandle::new()?;

    // канал кадров из capture
    let (tx, rx) = mpsc::channel::<(u32, u32, Vec<u8>)>();

    let ui_weak = ui.app.as_weak();

    // Поток для UI отображения кадров
    thread::spawn(move || {
        while let Ok((w, h, data)) = rx.recv() {
            let ui_weak = ui_weak.clone();
            slint::invoke_from_event_loop(move || {
                if let Some(app) = ui_weak.upgrade() {
                    let buffer = SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(&data, w, h);
                    let image = Image::from_rgba8(buffer);
                    app.set_preview(image);
                }
            })
                .ok();
        }
    });

    let windows_cache = Arc::new(Mutex::new(Vec::<WindowInfo>::new()));

    // Обновление списка окон
    {
        let cache = windows_cache.clone();
        let ui_ref = ui.app.as_weak();
        ui.app.on_request_titles(move || {
            let windows = get_windows();
            let titles: Vec<SharedString> =
                windows.iter().map(|w| SharedString::from(&w.title)).collect();

            *cache.lock().unwrap() = windows;

            if let Some(app) = ui_ref.upgrade() {
                app.set_titles((&titles[..]).into());
            }
        });
    }

    // Обработчик выбора окна
    {
        let cache = windows_cache.clone();
        let tx_capture = tx.clone();

        ui.app.on_window_selected(move |index| {
            let windows = cache.lock().unwrap();
            if let Some(selected) = windows.get(index as usize) {
                let hwnd_value = selected.hwnd.0 as isize;

                // Запускаем CaptureEngine в отдельном MTA-потоке
                let tx_capture = tx_capture.clone();
                thread::spawn(move || {
                    eprintln!("✓ Capture on_window_selected");
                    let hwnd = HWND(hwnd_value as _);
                    let engine = CaptureEngine::init().unwrap();

                    engine
                        .start(hwnd, move|| {
                            eprintln!("✓ Capture START");
                            // Пока просто создаём тестовый кадр
                            // Можно сюда впихнуть код для получения кадра и отправки по tx_capture
                            let width = 800;
                            let height = 600;
                            let data = vec![0u8; (width * height * 4)];
                            tx_capture.send((
                                width.try_into().unwrap(),
                                height.try_into().unwrap(),
                                data,
                            )).ok();
                        })
                        .unwrap();
                    //держим поток открытым
                    loop {
                        std::thread::sleep(std::time::Duration::from_secs(1));
                    }
                });
            }
        });
    }

    ui.app.run()?;
    Ok(())
}