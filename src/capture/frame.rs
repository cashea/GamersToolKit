//! Frame data structures for captured screen content

use std::time::Instant;

/// A captured frame from the screen
#[derive(Debug)]
pub struct CapturedFrame {
    /// Raw RGBA pixel data
    pub data: Vec<u8>,
    /// Frame width in pixels
    pub width: u32,
    /// Frame height in pixels
    pub height: u32,
    /// Timestamp when frame was captured
    pub timestamp: Instant,
}

impl CapturedFrame {
    /// Create a new captured frame
    pub fn new(data: Vec<u8>, width: u32, height: u32) -> Self {
        Self {
            data,
            width,
            height,
            timestamp: Instant::now(),
        }
    }

    /// Get frame dimensions as (width, height)
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }
}
