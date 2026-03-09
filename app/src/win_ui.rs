#![cfg(target_os = "windows")]

/*
    Windows UI Runtime

    This module connects the Slint UI with the Windows backend.

    Responsibilities:

    - Provide window list to the UI
    - Start / stop capture sessions
    - Transfer captured frames to the UI preview
    - Coordinate background threads

    High level pipeline:

        UI
         │
         ▼
    window selection
         │
         ▼
    CaptureEngine (windows-backend)
         │
         ▼
    frame channel (mpsc)
         │
         ▼
    UI thread (invoke_from_event_loop)
         │
         ▼
    Slint preview image
*/

use screen_ui::UiHandle;
use screen_ui::*;

use slint::{Image, Rgba8Pixel, SharedPixelBuffer, SharedString};

use std::sync::{mpsc, Arc, Mutex};
use std::thread;

use windows_backend::capture_engine::CaptureEngine;
use windows_backend::record_engine::RecordEngine;
use windows_backend::catcher::{get_windows, WindowInfo};

use windows::Win32::Foundation::HWND;

/// Commands sent to the capture worker thread.
///
/// These commands control the lifecycle of the capture engine.
enum UICommand {
    /// Start capturing a specific window.
    StartCapture(isize),
    /// Stop current capture session.
    StopCapture,
    /// Exit worker thread.
    Exit,
}

/// Entry point for the Windows UI runtime.
///
/// Initializes the UI, connects event handlers
/// and spawns background worker threads.
pub fn run() -> Result<(), Box<dyn std::error::Error>> {

    let ui = Arc::new(UiHandle::new()?);

    /*
        WINDOW CACHE
    */
    let windows_cache = Arc::new(Mutex::new(Vec::<WindowInfo>::new()));

    register_window_list_provider(&ui, windows_cache.clone());

    /*
        FRAME CHANNEL
    */
    let (frame_tx, frame_rx) = mpsc::channel::<(u32, u32, Vec<u8>)>();

    /*
        RECORD CHANNEL
    */
    let (record_tx, record_rx) = mpsc::channel::<Vec<u8>>();

    let record_rx = Arc::new(Mutex::new(Some(record_rx)));
    let record_engine = Arc::new(Mutex::new(RecordEngine::new()));

    spawn_ui_frame_consumer(ui.app.as_weak(), frame_rx, Some(record_tx.clone()));

    /*
        CAPTURE COMMAND WORKER
    */
    let (cmd_tx, cmd_rx) = mpsc::channel::<UICommand>();

    spawn_capture_worker(cmd_rx, frame_tx);

    /*
        UI EVENT HANDLERS
    */

    register_window_selection_handler(&ui, windows_cache.clone(), cmd_tx.clone());

    register_stop_capture_handler(&ui, cmd_tx.clone());

    register_start_record_handler(&ui, record_engine.clone(), record_rx.clone());

    register_stop_record_handler(&ui, record_engine.clone());

    ui.app.run()?;

    Ok(())
}

/*
    ============================================================
    UI EVENT REGISTRATION
    ============================================================
*/

fn register_window_list_provider(ui: &UiHandle, cache: Arc<Mutex<Vec<WindowInfo>>>) {
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

fn register_window_selection_handler(
    ui: &UiHandle,
    cache: Arc<Mutex<Vec<WindowInfo>>>,
    cmd_tx: mpsc::Sender<UICommand>,
) {
    ui.app.on_window_selected(move |index| {

        let windows = cache.lock().unwrap();

        if let Some(selected) = windows.get(index as usize) {

            let hwnd_value = selected.hwnd.0 as isize;

            cmd_tx.send(UICommand::StartCapture(hwnd_value)).ok();
        }
    });
}

fn register_stop_capture_handler(ui: &UiHandle, cmd_tx: mpsc::Sender<UICommand>) {
    ui.app.on_stop_capture(move || {

        cmd_tx.send(UICommand::StopCapture).ok();

        eprintln!("Button click STOP capture ==================>>>>>>>>>>>>");

    });
}

// register_start_record_handler
fn register_start_record_handler(
    ui: &UiHandle,
    record_engine: Arc<Mutex<RecordEngine>>,
    record_rx: Arc<Mutex<Option<mpsc::Receiver<Vec<u8>>>>>,
) {
    ui.app.on_start_record(move || {
        let rx = record_rx.lock().unwrap().take();
        if let Some(rx) = rx {
            let mut engine = record_engine.lock().unwrap();

            // Передаём ширину и высоту, как раньше
            engine.start_recording(
                1920,          // width
                1080,          // height
                30,            // fps
                "output.mp4".to_string(),
                rx,
            );
        }
    });
}

fn register_stop_record_handler(
    ui: &UiHandle,
    record_engine: Arc<Mutex<RecordEngine>>,
) {

    ui.app.on_stop_record(move || {

        let mut engine = record_engine.lock().unwrap();

        engine.stop_recording();

    });
}

/*
    ============================================================
    FRAME PIPELINE
    ============================================================
*/

fn spawn_ui_frame_consumer(
    ui: slint::Weak<AppWindow>,
    frame_rx: mpsc::Receiver<(u32, u32, Vec<u8>)>,
    record_tx: Option<mpsc::Sender<Vec<u8>>>,
)
{
    std::thread::spawn(move || {

        while let Ok((w, h, mut data)) = frame_rx.recv() {
            // Send original BGRA to recorder
            if let Some(tx) = &record_tx {
                tx.send(data.clone()).ok();
            }
            // Convert for UI preview
            convert_bgra_to_rgba(&mut data);

            let weak = ui.clone();

            // Send frame to UI thread
            slint::invoke_from_event_loop(move || {

                if let Some(app) = weak.upgrade() {

                    let buffer =
                        SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(&data, w, h);

                    let image = Image::from_rgba8(buffer);

                    app.set_preview(image);

                }

            }).ok();
        }

    });
}

/*
    ============================================================
    CAPTURE WORKER
    ============================================================
*/

fn spawn_capture_worker(
    cmd_rx: mpsc::Receiver<UICommand>,
    frame_tx: mpsc::Sender<(u32, u32, Vec<u8>)>,
) {

    thread::spawn(move || {

        use std::sync::atomic::{AtomicBool, Ordering};

        let mut running_flag: Option<Arc<AtomicBool>> = None;

        while let Ok(cmd) = cmd_rx.recv() {

            match cmd {

                UICommand::StartCapture(hwnd_value) => {

                    start_capture(hwnd_value, frame_tx.clone(), &mut running_flag);

                }

                UICommand::StopCapture => {

                    stop_capture(&mut running_flag);

                }

                UICommand::Exit => break,
            }
        }
    });
}

fn start_capture(
    hwnd_value: isize,
    frame_tx: mpsc::Sender<(u32, u32, Vec<u8>)>,
    running_flag: &mut Option<Arc<std::sync::atomic::AtomicBool>>,
) {

    use std::sync::atomic::AtomicBool;

    let running = Arc::new(AtomicBool::new(true));

    *running_flag = Some(running.clone());

    std::thread::spawn(move || {

        let hwnd = HWND(hwnd_value as _);

        let mut engine = CaptureEngine::init().unwrap();

        engine
            .start(hwnd, running, move |w, h, data| {

                println!("FRAME {}x{} bytes={}", w, h, data.len());

                frame_tx.send((w, h, data)).ok();

            })
            .ok();

    });
}

fn stop_capture(running_flag: &mut Option<Arc<std::sync::atomic::AtomicBool>>) {

    use std::sync::atomic::Ordering;

    if let Some(flag) = running_flag {

        flag.store(false, Ordering::SeqCst);

    }

    *running_flag = None;
}

/// Converts BGRA pixel buffer into RGBA.
///
/// Windows Graphics Capture produces frames in
/// `DXGI_FORMAT_B8G8R8A8_UNORM`.
///
/// Slint expects `RGBA8`.
pub fn convert_bgra_to_rgba(data: &mut [u8]) {
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
}