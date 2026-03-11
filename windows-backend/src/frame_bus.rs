use std::sync::{mpsc, Arc, Mutex};
use crate::frame::VideoFrame;

pub struct FrameBus {
    subs: Mutex<Vec<mpsc::Sender<Arc<VideoFrame>>>>,
}

impl FrameBus {
    pub fn new() -> Self {
        Self {
            subs: Mutex::new(Vec::new()),
        }
    }

    pub fn subscribe(&self) -> mpsc::Receiver<Arc<VideoFrame>> {
        let (tx, rx) = mpsc::channel();
        self.subs.lock().unwrap().push(tx);
        rx
    }

    pub fn publish(&self, frame: Arc<VideoFrame>) {
        let subs = self.subs.lock().unwrap();

        for s in subs.iter() {
            let _ = s.send(frame.clone());
        }
    }
}