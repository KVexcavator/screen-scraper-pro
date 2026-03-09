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
    /// Creates the main UI handle wrapped in an atomic reference counter.
    ///
    /// # Explanation
    /// - `UiHandle::new()` initializes the Slint `AppWindow` and internal models.
    /// - `Arc` (Atomic Reference Counted) allows safe cloning and sharing of this handle
    ///   between multiple threads without violating Rust's ownership rules.
    let ui = Arc::new(UiHandle::new()?);

    /*
        WINDOW CACHE

        We keep a cached list of enumerated windows.
        The UI only sees titles, but we keep HWNDs internally.
    */
    let windows_cache = Arc::new(Mutex::new(Vec::<WindowInfo>::new()));

    register_window_list_provider(&ui, windows_cache.clone());

    /*
        FRAME CHANNEL

        Frames produced by the capture engine are transferred
        to the UI thread using this channel.
    */
    let (frame_tx, frame_rx) = mpsc::channel::<(u32, u32, Vec<u8>)>();

    spawn_ui_frame_consumer(ui.app.as_weak(), frame_rx);

    /*
        CAPTURE COMMAND WORKER

        This thread owns the lifecycle of the capture engine.
    */
    let (cmd_tx, cmd_rx) = mpsc::channel::<UICommand>();

    spawn_capture_worker(cmd_rx, frame_tx);

    /*
        UI EVENT HANDLERS
    */

    register_window_selection_handler(&ui, windows_cache.clone(), cmd_tx.clone());

    register_stop_handler(&ui, cmd_tx.clone());

    ui.app.run()?;

    Ok(())
}

/*
    ============================================================
    UI EVENT REGISTRATION
    ============================================================
*/

/// Registers callback used by the UI to request window titles.
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

/// Registers handler triggered when a window is selected in the UI.
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

/// Registers stop capture button handler.
fn register_stop_handler(ui: &UiHandle, cmd_tx: mpsc::Sender<UICommand>) {
    ui.app.on_stop_capture(move || {
        cmd_tx.send(UICommand::StopCapture).ok();

        eprintln!("Button click STOP capture ==================>>>>>>>>>>>>");
    });
}

/*
    ============================================================
    FRAME PIPELINE
    ============================================================
*/

/// Spawns a thread responsible for delivering frames to the UI.
fn spawn_ui_frame_consumer(
    ui: slint::Weak<AppWindow>,
    frame_rx: mpsc::Receiver<(u32, u32, Vec<u8>)>,
) {
    std::thread::spawn(move || {
        while let Ok((w, h, data)) = frame_rx.recv() {
            let weak = ui.clone();

            slint::invoke_from_event_loop(move || {
                if let Some(app) = weak.upgrade() {
                    let image = frame_to_slint_image(w, h, data);
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

/// Spawns the capture command worker.
///
/// The worker listens for commands and manages the
/// lifetime of the capture engine.
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

/// Starts a new capture session.
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

/// Stops the currently running capture session.
fn stop_capture(running_flag: &mut Option<Arc<std::sync::atomic::AtomicBool>>) {
    use std::sync::atomic::Ordering;

    if let Some(flag) = running_flag {
        flag.store(false, Ordering::SeqCst);
    }

    *running_flag = None;
}

/*
    ============================================================
    FRAME → SLINT IMAGE
    ============================================================
*/

/// Converts raw RGBA frame buffer into a Slint Image.
///
/// The capture engine produces frames in RGBA format.
/// Slint expects `SharedPixelBuffer<Rgba8Pixel>`.
pub fn frame_to_slint_image(width: u32, height: u32, data: Vec<u8>) -> Image {
    let buffer = SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(&data, width, height);

    Image::from_rgba8(buffer)
}
