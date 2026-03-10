#![cfg(target_os = "windows")]

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

use windows::{
    Win32::{
        Media::Audio::*,
        System::Com::*,
    },
};

pub struct AudioSysEngine {
    running: Arc<AtomicBool>,
    handle: Option<thread::JoinHandle<()>>,
}

impl AudioSysEngine {

    pub fn new() -> Self {
        Self {
            running: Arc::new(AtomicBool::new(false)),
            handle: None,
        }
    }

    pub fn start(&mut self, path: String) {

        if self.running.load(Ordering::SeqCst) {
            return;
        }

        self.running.store(true, Ordering::SeqCst);
        let running = self.running.clone();

        self.handle = Some(thread::spawn(move || unsafe {

            CoInitializeEx(None, COINIT_MULTITHREADED).unwrap();

            let enumerator: IMMDeviceEnumerator =
                CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).unwrap();

            let device = enumerator
                .GetDefaultAudioEndpoint(eRender, eConsole)
                .unwrap();

            let audio_client: IAudioClient =
                device.Activate(CLSCTX_ALL, None).unwrap();

            let format_ptr: *mut WAVEFORMATEX =
                audio_client.GetMixFormat().unwrap();

            let format = *format_ptr;

            // копируем значения из packed struct
            let channels = format.nChannels;
            let sample_rate = format.nSamplesPerSec;

            audio_client.Initialize(
                AUDCLNT_SHAREMODE_SHARED,
                AUDCLNT_STREAMFLAGS_LOOPBACK,
                0,
                0,
                format_ptr,
                None,
            ).unwrap();

            let capture_client: IAudioCaptureClient =
                audio_client.GetService().unwrap();

            audio_client.Start().unwrap();

            let spec = hound::WavSpec {
                channels,
                sample_rate,
                bits_per_sample: 16,
                sample_format: hound::SampleFormat::Int,
            };

            let mut writer = hound::WavWriter::create(path, spec).unwrap();

            println!("SysAudio started {}Hz {}ch", sample_rate, channels);

            while running.load(Ordering::Relaxed) {

                let mut packet_size =
                    capture_client.GetNextPacketSize().unwrap_or(0);

                while packet_size > 0 {

                    let mut data_ptr = std::ptr::null_mut();
                    let mut frames = 0;
                    let mut flags = 0;

                    capture_client.GetBuffer(
                        &mut data_ptr,
                        &mut frames,
                        &mut flags,
                        None,
                        None,
                    ).unwrap();

                    let samples = std::slice::from_raw_parts(
                        data_ptr as *const f32,
                        frames as usize * channels as usize,
                    );

                    for &s in samples {
                        let v = (s * i16::MAX as f32) as i16;
                        writer.write_sample(v).ok();
                    }

                    capture_client.ReleaseBuffer(frames).ok();

                    packet_size =
                        capture_client.GetNextPacketSize().unwrap_or(0);
                }

                std::thread::sleep(std::time::Duration::from_millis(5));
            }

            audio_client.Stop().ok();

            writer.finalize().ok();

            println!("AudioSys stopped");

            CoUninitialize();
        }));
    }

    pub fn stop(&mut self) {

        if !self.running.load(Ordering::SeqCst) {
            return;
        }

        self.running.store(false, Ordering::SeqCst);

        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}