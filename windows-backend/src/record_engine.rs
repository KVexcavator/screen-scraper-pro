#![cfg(target_os = "windows")]

use std::sync::{Arc, mpsc::Receiver, atomic::{AtomicBool, Ordering}};
use std::thread;
use std::process::{Command, Stdio};
use std::io::Write;
use crate::bus::packets::VideoFrame;

pub struct RecordEngine {
    running: Arc<AtomicBool>,
    handle: Option<thread::JoinHandle<()>>,
}

impl RecordEngine {
    pub fn new() -> Self {
        Self {
            running: Arc::new(AtomicBool::new(false)),
            handle: None,
        }
    }

    /// width, height, fps → размеры кадра и частота
    /// path → куда сохраняем (временный BGRA .avi)
    /// frame_rx → поток кадров BGRA
    pub fn start_recording(
        &mut self,
        width: u32,
        height: u32,
        fps: u32,
        path: String,
        frame_rx: Receiver<Arc<VideoFrame>>,
    ) {
        if self.running.load(Ordering::SeqCst) {
            println!("RecordEngine: already running");
            return;
        }
        self.running.store(true, Ordering::SeqCst);
        let running = self.running.clone();

        self.handle = Some(thread::spawn(move || {
            let mut ffmpeg = Command::new(r".\bin\ffmpeg.exe")
                .args([
                    "-y",
                    "-f", "rawvideo",
                    "-pix_fmt", "rgba",
                    "-s", &format!("{}x{}", width, height),
                    "-r", &fps.to_string(),
                    "-i", "pipe:0",
                    "-c:v", "rawvideo",
                    &path,
                ])
                .stdin(Stdio::piped())
                .spawn()
                .expect("Failed to spawn ffmpeg");

            let mut stdin = ffmpeg.stdin.take().unwrap();

            while running.load(Ordering::SeqCst) {
                match frame_rx.recv() {
                    Ok(frame) => {
                        if stdin.write_all(frame.data.as_slice()).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }

            // закрываем stdin, ждем завершения ffmpeg
            drop(stdin);
            let _ = ffmpeg.wait();
        }));
    }

    pub fn stop_recording(&mut self) {
        if !self.running.load(Ordering::SeqCst) {
            return;
        }
        self.running.store(false, Ordering::SeqCst);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}