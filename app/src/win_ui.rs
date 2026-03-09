#![cfg(target_os = "windows")]
use screen_ui::UiHandle;
use screen_ui::*;
use slint::{Image, Rgba8Pixel, SharedPixelBuffer, SharedString};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use windows_backend::capture_engine::CaptureEngine;
use windows_backend::catcher::{WindowInfo, get_windows};
use windows::Win32::Foundation::HWND;

enum CaptureCommand {
    Start(isize),
    Stop,
    Exit,
}

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let ui = UiHandle::new()?;

    // Обновление списка окон
    let windows_cache = Arc::new(Mutex::new(Vec::<WindowInfo>::new()));
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

    // Поток для UI отображения кадров
    let ui_weak = ui.app.as_weak();
    let (frame_tx, frame_rx) = mpsc::channel::<(u32, u32, Vec<u8>)>();
    thread::spawn(move || {
        while let Ok((w, h, data)) = frame_rx.recv() {
            let ui_weak = ui_weak.clone();
            slint::invoke_from_event_loop(move || {
                if let Some(app) = ui_weak.upgrade() {
                    let buffer =
                        SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(&data, w, h);
                    let image = Image::from_rgba8(buffer);
                    app.set_preview(image);
                }
            }).ok();
        }
    });

    // Worker поток для команд(CaptureCommand)
    let (cmd_tx, cmd_rx) = mpsc::channel::<CaptureCommand>();
    thread::spawn(move || {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;

        let mut engine: Option<CaptureEngine> = None;
        let mut running_flag: Option<Arc<AtomicBool>> = None;

        while let Ok(cmd) = cmd_rx.recv() {
            match cmd {
                CaptureCommand::Start(hwnd_value) => {
                    let hwnd_value = hwnd_value; // уже isize

                    let running = Arc::new(AtomicBool::new(true));
                    running_flag = Some(running.clone());

                    let frame_tx_clone = frame_tx.clone();

                    std::thread::spawn(move || {
                        let hwnd = HWND(hwnd_value as _); // создаём тут

                        let mut engine = CaptureEngine::init().unwrap();

                        engine
                            .start(hwnd, running, move |w, h, data| {
                                println!("FRAME {}x{} bytes={}", w, h, data.len());
                                frame_tx_clone.send((w, h, data)).ok();
                            })
                            .ok();
                    });
                }

                CaptureCommand::Stop => {
                    if let Some(flag) = &running_flag {
                        flag.store(false, Ordering::SeqCst);
                    }
                    running_flag = None;
                }

                CaptureCommand::Exit => break,
            }
        }
    });

    // Обработчик выбора окна
    // фрейм стартует при выборе
    {
        let cache = windows_cache.clone();
        let tx = cmd_tx.clone();

        ui.app.on_window_selected(move |index| {
            let windows = cache.lock().unwrap();
            if let Some(selected) = windows.get(index as usize) {
                let hwnd_value = selected.hwnd.0 as isize;
                tx.send(CaptureCommand::Start(hwnd_value)).ok();
            }
        });
    }

    // Обработчик остановки фрейма
    {
        let tx = cmd_tx.clone();

        ui.app.on_stop_capture(move || {
            tx.send(CaptureCommand::Stop).ok();
            eprintln!("Button click STOP capture ==================>>>>>>>>>>>>");
        });
    }

    ui.app.run()?;
    Ok(())
}