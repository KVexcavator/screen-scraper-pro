#![cfg(target_os = "windows")]

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc::Receiver};
use std::thread;
use windows::{
    core::*,
    Win32::{
        Media::MediaFoundation::*,
        System::Com::*,
    },
};

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
    /// path → куда сохраняем
    /// frame_rx → поток кадров BGRA
    pub fn start_recording(
        &mut self,
        width: u32,
        height: u32,
        fps: u32,
        path: String,
        frame_rx: Receiver<Vec<u8>>,
    ) {
        if self.running.load(Ordering::SeqCst) {
            println!("MF: Recording already running");
            return;
        }

        self.running.store(true, Ordering::SeqCst);
        let running = self.running.clone();

        self.handle = Some(thread::spawn(move || unsafe {
            CoInitializeEx(None, COINIT_MULTITHREADED).unwrap();
            MFStartup(MF_VERSION, MFSTARTUP_FULL).unwrap();

            println!("MF: Creating SinkWriter at path {}", path);
            let writer = MFCreateSinkWriterFromURL(&HSTRING::from(path), None, None)
                .expect("MF: Failed to create SinkWriter");

            // =====================
            // OUTPUT TYPE (H264)
            // =====================
            let out_type = MFCreateMediaType().unwrap();
            out_type.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video).unwrap();
            out_type.SetGUID(&MF_MT_SUBTYPE, &MFVideoFormat_H264).unwrap();
            out_type.SetUINT32(&MF_MT_AVG_BITRATE, 8_000_000).unwrap();
            out_type
                .SetUINT32(&MF_MT_INTERLACE_MODE, MFVideoInterlace_Progressive.0 as u32)
                .unwrap();
            out_type
                .SetUINT64(&MF_MT_FRAME_SIZE, ((width as u64) << 32) | height as u64)
                .unwrap();
            out_type
                .SetUINT64(&MF_MT_FRAME_RATE, ((fps as u64) << 32) | 1)
                .unwrap();

            let stream_index = writer.AddStream(&out_type).unwrap();
            println!("MF: Stream index {}", stream_index);

            // =====================
            // INPUT TYPE (BGRA)
            // =====================
            let in_type = MFCreateMediaType().unwrap();
            in_type.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video).unwrap();
            in_type.SetGUID(&MF_MT_SUBTYPE, &MFVideoFormat_RGB32).unwrap();
            let stride = width * 4;
            println!("MF: Using stride {} bytes", stride);
            in_type.SetUINT32(&MF_MT_DEFAULT_STRIDE, stride).unwrap();
            in_type
                .SetUINT64(&MF_MT_FRAME_SIZE, ((width as u64) << 32) | height as u64)
                .unwrap();
            in_type
                .SetUINT64(&MF_MT_FRAME_RATE, ((fps as u64) << 32) | 1)
                .unwrap();

            writer.SetInputMediaType(stream_index, &in_type, None).unwrap();
            writer.BeginWriting().unwrap();

            let frame_duration = 10_000_000 / fps as i64;
            let mut rt = 0;
            let mut frame_count = 0;

            while running.load(Ordering::SeqCst) {
                let mut frame = match frame_rx.recv() {
                    Ok(f) => f,
                    Err(_) => break,
                };

                frame_count += 1;

                let expected_len = (stride * height) as usize;
                if frame.len() != expected_len {
                    println!(
                        "MF: WARNING: frame.len()={} != stride*height={} -> padding",
                        frame.len(),
                        expected_len
                    );
                    frame.resize(expected_len, 0);
                }

                let buffer = MFCreateMemoryBuffer(expected_len as u32).unwrap();
                let mut ptr: *mut u8 = std::ptr::null_mut();
                let mut max: u32 = 0;
                let mut cur: u32 = 0;

                buffer.Lock(&mut ptr, Some(&mut max), Some(&mut cur)).unwrap();
                std::ptr::copy_nonoverlapping(frame.as_ptr(), ptr, frame.len());
                buffer.Unlock().unwrap();
                buffer.SetCurrentLength(frame.len() as u32).unwrap();

                let sample = MFCreateSample().unwrap();
                sample.AddBuffer(&buffer).unwrap();
                sample.SetSampleTime(rt).unwrap();
                sample.SetSampleDuration(frame_duration).unwrap();

                if let Err(e) = writer.WriteSample(stream_index, &sample) {
                    println!("MF: WriteSample failed at frame {}: {:?}", frame_count, e);
                    break;
                }

                rt += frame_duration;
            }

            if let Err(e) = writer.Finalize() {
                println!("MF: Finalize failed: {:?}", e);
            }

            MFShutdown().unwrap();
            println!("MF: Recording finished, total frames={}", frame_count);
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