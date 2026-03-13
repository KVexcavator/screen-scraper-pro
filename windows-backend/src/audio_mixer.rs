#![cfg(target_os = "windows")]

use crate::bus::audio::AudioBus;
use crate::bus::packets::AudioPacket;

use std::sync::{Arc, mpsc};
use std::thread;
use std::time::Instant;

pub struct AudioMixer {
    mic_rx: mpsc::Receiver<Arc<AudioPacket>>,
    sys_rx: mpsc::Receiver<Arc<AudioPacket>>,
    bus: Arc<AudioBus>,
}

impl AudioMixer {

    pub fn new(
        mic_rx: mpsc::Receiver<Arc<AudioPacket>>,
        sys_rx: mpsc::Receiver<Arc<AudioPacket>>,
        bus: Arc<AudioBus>,
    ) -> Self {
        Self { mic_rx, sys_rx, bus }
    }

    pub fn start(self) {

        thread::spawn(move || {

            let mut mic_buf: Vec<f32> = Vec::new();
            let mut sys_buf: Vec<f32> = Vec::new();

            let mut channels = 2;
            let mut sample_rate = 48000;

            loop {

                if let Ok(pkt) = self.mic_rx.try_recv() {

                    mic_buf.extend_from_slice(&pkt.samples);
                    channels = pkt.channels;
                    sample_rate = pkt.sample_rate;

                }

                if let Ok(pkt) = self.sys_rx.try_recv() {

                    sys_buf.extend_from_slice(&pkt.samples);
                    channels = pkt.channels;
                    sample_rate = pkt.sample_rate;

                }

                let min_len = mic_buf.len().min(sys_buf.len());

                if min_len == 0 {
                    thread::sleep(std::time::Duration::from_millis(1));
                    continue;
                }

                let mut out = Vec::with_capacity(min_len);

                for i in 0..min_len {

                    let mixed =
                        (mic_buf[i] + sys_buf[i]) * 0.5;

                    out.push(mixed);

                }

                mic_buf.drain(..min_len);
                sys_buf.drain(..min_len);

                let packet = Arc::new(AudioPacket {
                    samples: Arc::new(out),
                    channels,
                    sample_rate,
                    timestamp: Instant::now(),
                });

                self.bus.publish(packet);
            }
        });
    }
}