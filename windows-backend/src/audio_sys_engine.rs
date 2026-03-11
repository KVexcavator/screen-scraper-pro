#![cfg(target_os = "windows")]
use crate::bus::audio::AudioBus;
use crate::bus::packets::AudioPacket;
use std::time::Instant;
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
    bus: Arc<AudioBus>,
}

impl AudioSysEngine {

    pub fn new(bus: Arc<AudioBus>) -> Self {
        Self {
            running: Arc::new(AtomicBool::new(false)),
            handle: None,
            bus,
        }
    }

    pub fn start(&mut self) {

        if self.running.load(Ordering::SeqCst) {
            return;
        }

        self.running.store(true, Ordering::SeqCst);
        let running = self.running.clone();
        let bus = self.bus.clone();

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

                    let packet = Arc::new(AudioPacket {
                        samples: Arc::new(samples.to_vec()),
                        channels: channels as u32,
                        sample_rate,
                        timestamp: Instant::now(),
                    });

                    bus.publish(packet);

                    capture_client.ReleaseBuffer(frames).ok();

                    packet_size =
                        capture_client.GetNextPacketSize().unwrap_or(0);
                }

                std::thread::sleep(std::time::Duration::from_millis(5));
            }

            audio_client.Stop().ok();
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