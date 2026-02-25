use screen_ui::*;
use screen_ui::UiHandle;
use windows_backend::catcher::{get_windows, WindowInfo};
use windows_backend::capture::start_capture;
use slint::{SharedString, SharedPixelBuffer, Rgba8Pixel, Image};
use std::sync::mpsc;
use std::thread;
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let ui = UiHandle::new()?;
    // канал кадров
    let (tx, rx) = mpsc::channel::<(u32, u32, Vec<u8>)>();

    let ui_weak = ui.app.as_weak();

    thread::spawn(move || {
        while let Ok((w, h, data)) = rx.recv() {
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

    let windows_cache =
        std::rc::Rc::new(std::cell::RefCell::new(Vec::<WindowInfo>::new()));

    {
        let cache = windows_cache.clone();
        let ui_ref = ui.app.as_weak();

        ui.app.on_request_titles(move || {
            let windows = get_windows();

            let titles: Vec<SharedString> =
                windows.iter().map(|w| SharedString::from(&w.title)).collect();

            *cache.borrow_mut() = windows;

            if let Some(app) = ui_ref.upgrade() {
                app.set_titles((&titles[..]).into());
            }
        });
    }

    {
        let cache = windows_cache.clone();
        let tx_capture = tx.clone();

        ui.app.on_window_selected(move |index| {
            let windows = cache.borrow();
            if let Some(selected) = windows.get(index as usize) {
                println!("Selected title: {}", selected.title);

                start_capture(selected.hwnd, tx_capture.clone());
            }
        });
    }

    ui.app.run()?;
    Ok(())
}