#![cfg(target_os = "windows")]

use std::{
    io::Write,
    process::{Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::Receiver,
        Arc,
    },
    thread,
    time::Duration,
};

use named_pipe::PipeOptions;

use crate::bus::packets::{AudioPacket, VideoFrame};

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

    pub fn start_recording(
        &mut self,
        width: u32,
        height: u32,
        fps: u32,
        path: String,
        frame_rx: Receiver<Arc<VideoFrame>>,
        audio_rx: Receiver<Arc<AudioPacket>>,
    ) {
        if self.running.load(Ordering::SeqCst) {
            println!("RecordEngine: already running");
            return;
        }
        self.running.store(true, Ordering::SeqCst);
        let running = self.running.clone();

        println!("Starting ffmpeg...");
        self.handle = Some(thread::spawn(move || {
            // Создаём named pipe для аудио
            let pipe_name = r"\\.\pipe\screen_audio";
            let audio_pipe = PipeOptions::new(pipe_name)
                .single()
                .unwrap();
            println!("Named pipe created: {}", pipe_name);

            // Запускаем FFmpeg асинхронно
            let mut ffmpeg = Command::new(r".\bin\ffmpeg.exe")
                .stderr(Stdio::inherit())
                .stdout(Stdio::inherit())
                .stdin(Stdio::piped())
                .args([
                    "-y",
                    "-f", "rawvideo",
                    "-pix_fmt", "rgba",
                    "-s", &format!("{}x{}", width, height),
                    "-r", &fps.to_string(),
                    "-i", "pipe:0",
                    "-f", "f32le",
                    "-ar", "44100",
                    "-ac", "2",
                    "-i", pipe_name,
                    "-c:v", "libx264",
                    "-preset", "veryfast",
                    "-pix_fmt", "yuv420p",
                    "-c:a", "aac",
                    &path,
                ])
                .spawn()
                .expect("Failed to spawn ffmpeg");

            // Поток для видео
            let mut video_stdin = ffmpeg.stdin.take().unwrap();
            let running_v = running.clone();
            let video_thread = thread::spawn(move || {
                while running_v.load(Ordering::SeqCst) {
                    match frame_rx.recv_timeout(Duration::from_millis(50)) {
                        Ok(frame) => {
                            if video_stdin.write_all(&frame.data).is_err() {
                                break;
                            }
                        }
                        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
                        Err(_) => break,
                    }
                }
                // Закрываем stdin, чтобы FFmpeg завершился корректно
                let _ = video_stdin.flush();
            });

            // Поток для аудио
            let running_a = running.clone();
            let audio_thread = thread::spawn(move || {
                println!("Waiting for FFmpeg to connect to audio pipe...");
                let mut audio_writer = audio_pipe.wait().unwrap(); // <- wait() блокирует этот поток, но не основной
                println!("FFmpeg connected to audio pipe");

                while running_a.load(Ordering::SeqCst) {
                    match audio_rx.recv_timeout(Duration::from_millis(50)) {
                        Ok(pkt) => {
                            let bytes = unsafe {
                                std::slice::from_raw_parts(pkt.samples.as_ptr() as *const u8, pkt.samples.len() * 4)
                            };
                            if audio_writer.write_all(bytes).is_err() {
                                break;
                            }
                        }
                        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
                        Err(_) => break,
                    }
                }
                let _ = audio_writer.flush();
            });

            // Ждём оба потока
            let _ = video_thread.join();
            let _ = audio_thread.join();

            // Завершаем FFmpeg
            let _ = ffmpeg.wait();
            println!("FFmpeg finished");
        }));
    }

    pub fn stop_recording(&mut self) {
        if !self.running.load(Ordering::SeqCst) {
            return;
        }
        self.running.store(false, Ordering::SeqCst);

        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }

        println!("Recording stopped");
    }
}