use std::sync::Arc;
use std::time::Instant;

pub struct VideoFrame {
    pub width: u32,
    pub height: u32,
    pub data: Arc<Vec<u8>>,
    pub timestamp: Instant,
}

pub struct AudioPacket {
    pub samples: Arc<Vec<f32>>,
    pub channels: u32,
    pub sample_rate: u32,
    pub timestamp: Instant,
}