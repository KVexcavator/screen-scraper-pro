#![cfg(target_os = "windows")]

use screen_ui::UiHandle;
use screen_ui::*;
use slint::{Image, Rgba8Pixel, SharedPixelBuffer, SharedString};

use std::process::Command;
use std::sync::{mpsc, Arc, Mutex};
use std::sync::atomic::AtomicBool;
use std::thread;
use std::time::Instant;

use windows_backend::capture_engine::CaptureEngine;
use windows_backend::record_engine::RecordEngine;
use windows_backend::audio_mic_engine::AudioMicEngine;
use windows_backend::audio_sys_engine::AudioSysEngine;
use windows_backend::audio_mixer::AudioMixer;
use windows_backend::catcher::{get_windows, WindowInfo};
use windows_backend::bus::{
    packets::{VideoFrame, AudioPacket},
    frame::FrameBus,
    audio::AudioBus,
    audio_source::AudioSourceBus,
};

use windows::Win32::Foundation::HWND;


enum UICommand {
    StartCapture(isize),
    StopCapture,
    Exit,
}

pub fn run() -> Result<(), Box<dyn std::error::Error>> {

    let ui = Arc::new(UiHandle::new()?);
    // WINDOW CACHE
    let windows_cache = Arc::new(Mutex::new(Vec::<WindowInfo>::new()));
    register_window_list_provider(&ui, windows_cache.clone());
    // FRAME BUS
    let frame_bus = Arc::new(FrameBus::new());
    // AUDIO BUS
    let audio_bus = Arc::new(AudioBus::new());
    // SOURCE BUSES
    let mic_bus = Arc::new(AudioSourceBus::new());
    let sys_bus = Arc::new(AudioSourceBus::new());
    // MIXER
    let mixer = AudioMixer::new(
        mic_bus.subscribe(),
        sys_bus.subscribe(),
        audio_bus.clone(),
    );
    mixer.start();
    // RECORD BUS
    let record_frames = frame_bus.subscribe();
    let record_engine = Arc::new(Mutex::new(RecordEngine::new()));
    // AUDIO ENGINES
    let mic_engine =
        Arc::new(Mutex::new(AudioMicEngine::new(mic_bus.clone())));
    let sys_engine =
        Arc::new(Mutex::new(AudioSysEngine::new(sys_bus.clone())));

    spawn_ui_frame_consumer(
        ui.app.as_weak(),
        frame_bus.subscribe(),
    );

    // CAPTURE COMMAND WORKER
    let (cmd_tx, cmd_rx) = mpsc::channel::<UICommand>();

    spawn_capture_worker(cmd_rx, frame_bus.clone());

    // UI EVENT HANDLERS
    register_window_selection_handler(&ui, windows_cache.clone(), cmd_tx.clone());
    register_stop_capture_handler(&ui, cmd_tx.clone());
    register_start_record_handler(
        &ui,
        record_engine.clone(),
        frame_bus.clone(),
        audio_bus.clone(),
        mic_engine.clone(),
        sys_engine.clone(),
    );
    register_stop_record_handler(
        &ui,
        record_engine.clone(),
        mic_engine.clone(),
        sys_engine.clone(),
    );

    ui.app.run()?;
    Ok(())
}

// UI EVENT REGISTRATION
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

    });
}

// RECORD HANDLERS
fn register_start_record_handler(
    ui: &UiHandle,
    record_engine: Arc<Mutex<RecordEngine>>,
    frame_bus: Arc<FrameBus>,
    audio_bus: Arc<AudioBus>,
    mic_engine: Arc<Mutex<AudioMicEngine>>,
    sys_engine: Arc<Mutex<AudioSysEngine>>,
) {
    ui.app.on_start_record(move || {

        println!("Start recording");

        let rx = frame_bus.subscribe();

        let mut engine = record_engine.lock().unwrap();

        let audio_rx = audio_bus.subscribe();

        engine.start_recording(
            1920,
            1032,
            30,
            "final_output.mp4".to_string(),
            rx,
            audio_rx,
        );

        mic_engine.lock().unwrap().start();
        sys_engine.lock().unwrap().start();
    });
}

fn register_stop_record_handler(
    ui: &UiHandle,
    record_engine: Arc<Mutex<RecordEngine>>,
    mic_engine: Arc<Mutex<AudioMicEngine>>,
    sys_engine: Arc<Mutex<AudioSysEngine>>,
) {
    ui.app.on_stop_record(move || {

        println!("Stop recording");

        record_engine.lock().unwrap().stop_recording();

        mic_engine.lock().unwrap().stop();
        sys_engine.lock().unwrap().stop();

    });
}

// FRAME PIPELINE
fn spawn_ui_frame_consumer(
    ui: slint::Weak<AppWindow>,
    frame_rx: mpsc::Receiver<Arc<VideoFrame>>,
)
{
    std::thread::spawn(move || {

        while let Ok(frame) = frame_rx.recv() {

            let w = frame.width;
            let h = frame.height;
            let data = frame.data.clone();

            let weak = ui.clone();
            let preview = (*data).clone();

            slint::invoke_from_event_loop(move || {

                if let Some(app) = weak.upgrade() {

                    let buffer =
                        SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(&preview, w, h);

                    let image = Image::from_rgba8(buffer);

                    app.set_preview(image);

                }

            }).ok();
        }

    });
}


// CAPTURE WORKER
fn spawn_capture_worker(
    cmd_rx: mpsc::Receiver<UICommand>,
    frame_bus: Arc<FrameBus>,
) {

    thread::spawn(move || {

        use std::sync::atomic::{AtomicBool, Ordering};

        let mut running_flag: Option<Arc<AtomicBool>> = None;

        while let Ok(cmd) = cmd_rx.recv() {

            match cmd {

                UICommand::StartCapture(hwnd_value) => {

                    start_capture(hwnd_value, frame_bus.clone(), &mut running_flag);

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
    frame_bus: Arc<FrameBus>,
    running_flag: &mut Option<Arc<AtomicBool>>,
) {

    use std::sync::atomic::AtomicBool;

    let running = Arc::new(AtomicBool::new(true));

    *running_flag = Some(running.clone());

    std::thread::spawn(move || {

        let hwnd = HWND(hwnd_value as _);

        let mut engine = CaptureEngine::init().unwrap();

        engine
            .start(hwnd, running, move |w, h, mut data| {

                convert_bgra_to_rgba(&mut data);

                let frame = Arc::new(VideoFrame {
                    width: w,
                    height: h,
                    data: Arc::new(data),
                    timestamp: Instant::now(),
                });

                frame_bus.publish(frame);

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