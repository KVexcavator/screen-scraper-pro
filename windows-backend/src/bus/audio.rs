use std::sync::{mpsc, Arc, Mutex};
use crate::bus::packets::AudioPacket;

pub struct AudioBus {
    subs: Mutex<Vec<mpsc::Sender<Arc<AudioPacket>>>>,
}

impl AudioBus {
    pub fn new() -> Self {
        Self {
            subs: Mutex::new(Vec::new()),
        }
    }

    pub fn subscribe(&self) -> mpsc::Receiver<Arc<AudioPacket>> {

        let (tx, rx) = mpsc::channel();

        self.subs.lock().unwrap().push(tx);

        rx
    }

    pub fn publish(&self, packet: Arc<AudioPacket>) {

        let subs = self.subs.lock().unwrap();

        for s in subs.iter() {
            let _ = s.send(packet.clone());
        }

    }
}