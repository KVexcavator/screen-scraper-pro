use std::sync::Arc;
use std::time::Instant;

pub struct VideoFrame {
    pub width: u32,
    pub height: u32,
    pub data: Arc<Vec<u8>>,
    pub timestamp: Instant,
}