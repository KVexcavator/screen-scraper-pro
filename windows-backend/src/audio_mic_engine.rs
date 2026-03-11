#![cfg(target_os = "windows")]
// пишет только микрофон
use crate::bus::audio::AudioBus;
use crate::bus::packets::AudioPacket;
use std::time::Instant;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

pub struct AudioMicEngine {
    running: Arc<AtomicBool>,
    handle: Option<thread::JoinHandle<()>>,
    bus: Arc<AudioBus>,
}

impl AudioMicEngine {
    pub fn new(bus: Arc<AudioBus>) -> Self {
        Self {
            running: Arc::new(AtomicBool::new(false)),
            handle: None,
            bus,
        }
    }

    pub fn start(&mut self) {
        if self.running.load(Ordering::SeqCst) {
            println!("AudioEngine: already running");
            return;
        }

        self.running.store(true, Ordering::SeqCst);

        let running_thread = self.running.clone();
        let bus = self.bus.clone();

        self.handle = Some(thread::spawn(move || {

            let host = cpal::default_host();

            let device = host
                .default_input_device()
                .expect("AudioEngine: no input device");

            let config = device
                .default_input_config()
                .expect("AudioEngine: failed to get input config");

            let sample_rate = config.sample_rate().0;
            let channels = config.channels();

            println!(
                "AudioEngine: sample_rate={} channels={}",
                sample_rate, channels
            );

            let running_stream = running_thread.clone();

            let err_fn = |err| eprintln!("AudioEngine stream error: {}", err);

            let stream = device
                .build_input_stream(
                    &config.into(),
                    move |data: &[f32], _| {
                        if !running_stream.load(Ordering::Relaxed) {
                            return;
                        }

                        let packet = Arc::new(AudioPacket {
                            samples: Arc::new(data.to_vec()),
                            channels: channels as u32,
                            sample_rate,
                            timestamp: Instant::now(),
                        });

                        bus.publish(packet);
                    },
                    err_fn,
                )
                .expect("AudioMicEngine: build stream failed");

            stream.play().expect("AudioMicEngine: play failed");

            println!("AudioMicEngine: recording started");

            while running_thread.load(Ordering::Relaxed) {
                std::thread::sleep(std::time::Duration::from_millis(100));
            }

            println!("AudioMicEngine: stopping");

            drop(stream);

            println!("AudioMicEngine: finished");
        }));
    }

    pub fn stop(&mut self) {
        if !self.running.load(Ordering::SeqCst) {
            return;
        }

        self.running.store(false, Ordering::SeqCst);

        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }

        println!("AudioMicEngine: stopped");
    }
}