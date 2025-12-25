//! Screen Capture Layer
//!
//! Uses Windows Graphics Capture API for safe, anti-cheat compliant screen capture.
//! This is a read-only operation that captures pixels without any game interaction.

pub mod frame;

use anyhow::Result;

/// Screen capture configuration
#[derive(Debug, Clone)]
pub struct CaptureConfig {
    /// Target window title or process name
    pub target: String,
    /// Maximum frames per second to capture
    pub max_fps: u32,
    /// Whether to capture cursor
    pub capture_cursor: bool,
}

impl Default for CaptureConfig {
    fn default() -> Self {
        Self {
            target: String::new(),
            max_fps: 30,
            capture_cursor: false,
        }
    }
}

/// Screen capture manager using Windows Graphics Capture API
pub struct ScreenCapture {
    config: CaptureConfig,
    // TODO: Add windows-capture integration
}

impl ScreenCapture {
    /// Create a new screen capture instance
    pub fn new(config: CaptureConfig) -> Result<Self> {
        Ok(Self { config })
    }

    /// Start capturing frames
    pub fn start(&mut self) -> Result<()> {
        // TODO: Initialize windows-capture
        Ok(())
    }

    /// Stop capturing
    pub fn stop(&mut self) -> Result<()> {
        // TODO: Stop capture
        Ok(())
    }
}
