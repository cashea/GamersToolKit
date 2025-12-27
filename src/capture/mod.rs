//! Screen Capture Layer
//!
//! Uses Windows Graphics Capture API for safe, anti-cheat compliant screen capture.
//! This is a read-only operation that captures pixels without any game interaction.

pub mod frame;

use anyhow::{Context, Result};
use crossbeam_channel::{Receiver, Sender, bounded};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tracing::{debug, error, info, warn};
use windows_capture::{
    capture::{Context as CaptureContext, GraphicsCaptureApiHandler},
    frame::Frame,
    graphics_capture_api::InternalCaptureControl,
    monitor::Monitor,
    settings::{
        ColorFormat, CursorCaptureSettings, DrawBorderSettings,
        Settings, SecondaryWindowSettings, MinimumUpdateIntervalSettings,
        DirtyRegionSettings,
    },
    window::Window,
};

use crate::capture::frame::CapturedFrame;

/// Screen capture configuration
#[derive(Debug, Clone)]
pub struct CaptureConfig {
    /// Target window title (partial match supported)
    pub target: CaptureTarget,
    /// Maximum frames per second to capture
    pub max_fps: u32,
    /// Whether to capture cursor
    pub capture_cursor: bool,
    /// Whether to draw border around captured window
    pub draw_border: bool,
}

/// What to capture
#[derive(Debug, Clone)]
pub enum CaptureTarget {
    /// Capture a specific window by title (partial match)
    Window(String),
    /// Capture primary monitor
    PrimaryMonitor,
    /// Capture monitor by index
    MonitorIndex(usize),
}

impl Default for CaptureConfig {
    fn default() -> Self {
        Self {
            target: CaptureTarget::PrimaryMonitor,
            max_fps: 30,
            capture_cursor: false,
            draw_border: false,
        }
    }
}

/// Screen capture manager using Windows Graphics Capture API
pub struct ScreenCapture {
    config: CaptureConfig,
    running: Arc<AtomicBool>,
    frame_receiver: Option<Receiver<CapturedFrame>>,
}

impl ScreenCapture {
    /// Create a new screen capture instance
    pub fn new(config: CaptureConfig) -> Result<Self> {
        Ok(Self {
            config,
            running: Arc::new(AtomicBool::new(false)),
            frame_receiver: None,
        })
    }

    /// List available windows for capture
    pub fn list_windows() -> Result<Vec<String>> {
        let windows = Window::enumerate().context("Failed to enumerate windows")?;
        Ok(windows
            .iter()
            .filter_map(|w| w.title().ok())
            .filter(|t| !t.is_empty())
            .collect())
    }

    /// List available monitors for capture
    pub fn list_monitors() -> Result<Vec<String>> {
        let monitors = Monitor::enumerate().context("Failed to enumerate monitors")?;
        Ok(monitors
            .iter()
            .enumerate()
            .map(|(i, m)| {
                let name = m.name().unwrap_or_else(|_| format!("Monitor {}", i));
                format!("{}: {}", i, name)
            })
            .collect())
    }

    /// Start capturing frames (non-blocking, spawns capture thread)
    pub fn start(&mut self) -> Result<()> {
        if self.running.load(Ordering::SeqCst) {
            warn!("Capture already running");
            return Ok(());
        }

        let (tx, rx) = bounded::<CapturedFrame>(2); // Small buffer to avoid memory buildup
        self.frame_receiver = Some(rx);
        self.running.store(true, Ordering::SeqCst);

        let config = self.config.clone();
        let running = self.running.clone();

        std::thread::spawn(move || {
            if let Err(e) = run_capture(config, tx, running.clone()) {
                error!("Capture error: {}", e);
            }
            running.store(false, Ordering::SeqCst);
        });

        info!("Screen capture started");
        Ok(())
    }

    /// Stop capturing
    pub fn stop(&mut self) -> Result<()> {
        self.running.store(false, Ordering::SeqCst);
        self.frame_receiver = None;
        info!("Screen capture stopped");
        Ok(())
    }

    /// Check if capture is running
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Get the next captured frame (blocks until available or capture stops)
    pub fn next_frame(&self) -> Option<CapturedFrame> {
        self.frame_receiver.as_ref()?.recv().ok()
    }

    /// Try to get the next captured frame without blocking
    pub fn try_next_frame(&self) -> Option<CapturedFrame> {
        self.frame_receiver.as_ref()?.try_recv().ok()
    }
}

/// Flags passed to the capture handler
struct CaptureFlags {
    frame_sender: Sender<CapturedFrame>,
    running: Arc<AtomicBool>,
    frame_interval_ms: u64,
}

/// Internal capture handler for windows-capture
struct CaptureHandler {
    frame_sender: Sender<CapturedFrame>,
    running: Arc<AtomicBool>,
    frame_interval_ms: u64,
    last_frame_time: std::time::Instant,
}

impl GraphicsCaptureApiHandler for CaptureHandler {
    type Flags = CaptureFlags;
    type Error = anyhow::Error;

    fn new(ctx: CaptureContext<Self::Flags>) -> Result<Self, Self::Error> {
        let flags = ctx.flags;
        Ok(Self {
            frame_sender: flags.frame_sender,
            running: flags.running,
            frame_interval_ms: flags.frame_interval_ms,
            last_frame_time: std::time::Instant::now(),
        })
    }

    fn on_frame_arrived(
        &mut self,
        frame: &mut Frame,
        capture_control: InternalCaptureControl,
    ) -> Result<(), Self::Error> {
        // Check if we should stop
        if !self.running.load(Ordering::SeqCst) {
            capture_control.stop();
            return Ok(());
        }

        // Rate limiting
        let elapsed = self.last_frame_time.elapsed().as_millis() as u64;
        if elapsed < self.frame_interval_ms {
            return Ok(());
        }
        self.last_frame_time = std::time::Instant::now();

        // Get frame buffer
        let width = frame.width();
        let height = frame.height();

        // Convert frame to RGBA buffer
        let mut buffer = frame.buffer().context("Failed to get frame buffer")?;
        let data = buffer.as_raw_buffer().to_vec();

        // Create captured frame (windows-capture uses BGRA, we'll convert later if needed)
        let captured = CapturedFrame::new_bgra(data, width, height);

        // Send frame (non-blocking, drop if receiver is full)
        if self.frame_sender.try_send(captured).is_err() {
            debug!("Frame dropped (receiver full)");
        }

        Ok(())
    }

    fn on_closed(&mut self) -> Result<(), Self::Error> {
        info!("Capture source closed");
        self.running.store(false, Ordering::SeqCst);
        Ok(())
    }
}

/// Run the capture loop (blocking)
fn run_capture(
    config: CaptureConfig,
    tx: Sender<CapturedFrame>,
    running: Arc<AtomicBool>,
) -> Result<()> {
    let frame_interval_ms = 1000 / config.max_fps.max(1) as u64;

    let cursor_settings = if config.capture_cursor {
        CursorCaptureSettings::WithCursor
    } else {
        CursorCaptureSettings::WithoutCursor
    };

    let border_settings = if config.draw_border {
        DrawBorderSettings::WithBorder
    } else {
        DrawBorderSettings::WithoutBorder
    };

    let flags = CaptureFlags {
        frame_sender: tx,
        running,
        frame_interval_ms,
    };

    match config.target {
        CaptureTarget::Window(title) => {
            let windows = Window::enumerate().context("Failed to enumerate windows")?;
            let window = windows
                .into_iter()
                .find(|w| {
                    w.title()
                        .map(|t| t.to_lowercase().contains(&title.to_lowercase()))
                        .unwrap_or(false)
                })
                .context(format!("Window '{}' not found", title))?;

            info!("Capturing window: {:?}", window.title());

            let settings = Settings::new(
                window,
                cursor_settings,
                border_settings,
                SecondaryWindowSettings::Default,
                MinimumUpdateIntervalSettings::Default,
                DirtyRegionSettings::Default,
                ColorFormat::Bgra8,
                flags,
            );

            CaptureHandler::start(settings).context("Failed to start window capture")?;
        }
        CaptureTarget::PrimaryMonitor => {
            let monitor = Monitor::primary().context("Failed to get primary monitor")?;
            info!("Capturing primary monitor: {:?}", monitor.name());

            let settings = Settings::new(
                monitor,
                cursor_settings,
                border_settings,
                SecondaryWindowSettings::Default,
                MinimumUpdateIntervalSettings::Default,
                DirtyRegionSettings::Default,
                ColorFormat::Bgra8,
                flags,
            );

            CaptureHandler::start(settings).context("Failed to start monitor capture")?;
        }
        CaptureTarget::MonitorIndex(idx) => {
            let monitors = Monitor::enumerate().context("Failed to enumerate monitors")?;
            let monitor = monitors
                .into_iter()
                .nth(idx)
                .context(format!("Monitor index {} not found", idx))?;

            info!("Capturing monitor {}: {:?}", idx, monitor.name());

            let settings = Settings::new(
                monitor,
                cursor_settings,
                border_settings,
                SecondaryWindowSettings::Default,
                MinimumUpdateIntervalSettings::Default,
                DirtyRegionSettings::Default,
                ColorFormat::Bgra8,
                flags,
            );

            CaptureHandler::start(settings).context("Failed to start monitor capture")?;
        }
    }

    Ok(())
}
