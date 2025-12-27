//! Overlay Presentation Layer
//!
//! Displays tips and alerts using egui_overlay with click passthrough.
//! The overlay is a separate window that doesn't interact with the game.

pub mod widgets;

use anyhow::Result;
use crossbeam_channel::{Receiver, Sender, unbounded};
use egui::{Align2, Color32, FontId, RichText, Rounding, Vec2};
use egui_overlay::{EguiOverlay, egui_window_glfw_passthrough::GlfwBackend, egui_render_three_d::ThreeDBackend};
use egui_overlay::egui_window_glfw_passthrough::glfw;
use parking_lot::RwLock;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::info;

use crate::analysis::Tip;
use crate::overlay::widgets::{PriorityStyles, TipStyle};

/// Overlay configuration
#[derive(Debug, Clone)]
pub struct OverlayConfig {
    /// Overlay opacity (0.0 - 1.0)
    pub opacity: f32,
    /// Whether overlay is enabled
    pub enabled: bool,
    /// Position offset from corner
    pub offset: (i32, i32),
    /// Which corner to anchor to
    pub anchor: OverlayAnchor,
    /// Maximum number of tips to show at once
    pub max_tips: usize,
    /// Default tip duration in milliseconds
    pub default_duration_ms: u64,
    /// Monitor index to display overlay on (0 = primary, None = auto-detect)
    pub monitor_index: Option<usize>,
}

/// Information about a connected monitor
#[derive(Debug, Clone)]
pub struct MonitorInfo {
    /// Monitor index (0-based)
    pub index: usize,
    /// Monitor name (if available)
    pub name: Option<String>,
    /// Position on virtual screen (x, y)
    pub position: (i32, i32),
    /// Work area (x, y, width, height) - excludes taskbar
    pub work_area: (i32, i32, i32, i32),
    /// Whether this is the primary monitor
    pub is_primary: bool,
}

impl Default for OverlayConfig {
    fn default() -> Self {
        Self {
            opacity: 0.9,
            enabled: true,
            offset: (20, 20),
            anchor: OverlayAnchor::TopRight,
            max_tips: 5,
            default_duration_ms: 5000,
            monitor_index: Some(0), // Default to primary monitor
        }
    }
}

/// Corner anchor for overlay positioning
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OverlayAnchor {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

/// A tip with timing information for display
#[derive(Debug, Clone)]
struct DisplayTip {
    tip: Tip,
    shown_at: Instant,
    expires_at: Option<Instant>,
}

impl DisplayTip {
    fn new(tip: Tip, default_duration_ms: u64) -> Self {
        let shown_at = Instant::now();
        let expires_at = tip.duration_ms
            .or(Some(default_duration_ms))
            .map(|ms| shown_at + Duration::from_millis(ms));
        Self {
            tip,
            shown_at,
            expires_at,
        }
    }

    fn is_expired(&self) -> bool {
        self.expires_at.map(|t| Instant::now() > t).unwrap_or(false)
    }

    fn age_secs(&self) -> f32 {
        self.shown_at.elapsed().as_secs_f32()
    }
}

/// Shared state between overlay thread and main application
pub struct OverlayState {
    tips: Vec<DisplayTip>,
    config: OverlayConfig,
    styles: PriorityStyles,
}

impl OverlayState {
    fn new(config: OverlayConfig) -> Self {
        Self {
            tips: Vec::new(),
            config,
            styles: PriorityStyles::default(),
        }
    }
}

/// Enumerate all connected monitors
///
/// Returns a list of monitor information that can be used to select
/// which monitor the overlay should appear on.
///
/// # Example
/// ```no_run
/// use gamers_toolkit::overlay::list_monitors;
///
/// let monitors = list_monitors();
/// for monitor in &monitors {
///     println!("{}: {} ({}x{} at {},{})",
///         monitor.index,
///         monitor.name.as_deref().unwrap_or("Unknown"),
///         monitor.work_area.2, monitor.work_area.3,
///         monitor.work_area.0, monitor.work_area.1
///     );
/// }
/// ```
pub fn list_monitors() -> Vec<MonitorInfo> {
    let mut glfw_instance = match glfw::init(glfw::fail_on_errors) {
        Ok(g) => g,
        Err(e) => {
            tracing::error!("Failed to initialize GLFW for monitor enumeration: {}", e);
            return Vec::new();
        }
    };

    let mut monitors = Vec::new();

    glfw_instance.with_connected_monitors(|_, connected| {
        for (index, monitor) in connected.iter().enumerate() {
            let (x, y, w, h) = monitor.get_workarea();
            let (px, py) = monitor.get_pos();

            monitors.push(MonitorInfo {
                index,
                name: monitor.get_name(),
                position: (px, py),
                work_area: (x, y, w, h),
                is_primary: index == 0, // First monitor is always primary in GLFW
            });
        }
    });

    monitors
}

/// Overlay window manager
pub struct OverlayManager {
    state: Arc<RwLock<OverlayState>>,
    tip_sender: Sender<Tip>,
    tip_receiver: Receiver<Tip>,
}

impl OverlayManager {
    /// Create a new overlay manager
    pub fn new(config: OverlayConfig) -> Result<Self> {
        let (tip_sender, tip_receiver) = unbounded();
        Ok(Self {
            state: Arc::new(RwLock::new(OverlayState::new(config))),
            tip_sender,
            tip_receiver,
        })
    }

    /// Get a sender for adding tips from other threads
    pub fn tip_sender(&self) -> Sender<Tip> {
        self.tip_sender.clone()
    }

    /// Show a tip on the overlay
    pub fn show_tip(&self, tip: Tip) {
        let _ = self.tip_sender.send(tip);
    }

    /// Clear all tips
    pub fn clear_tips(&self) {
        self.state.write().tips.clear();
    }

    /// Update config
    pub fn set_config(&self, config: OverlayConfig) {
        self.state.write().config = config;
    }

    /// Set the monitor index for the overlay
    ///
    /// Note: This must be called before `run()` to take effect.
    /// Use `list_monitors()` to get available monitor indices.
    pub fn set_monitor(&self, monitor_index: Option<usize>) {
        self.state.write().config.monitor_index = monitor_index;
    }

    /// Run the overlay event loop (blocking)
    /// This should be called from the main thread
    pub fn run(&self) -> Result<()> {
        info!("Starting overlay...");

        let state = self.state.clone();
        let tip_receiver = self.tip_receiver.clone();

        // Create the overlay app
        let app = OverlayApp {
            state,
            tip_receiver,
            positioned: false,
            monitor_bounds: None,
        };

        // Run egui_overlay
        egui_overlay::start(app);

        Ok(())
    }
}

/// The egui overlay application
struct OverlayApp {
    state: Arc<RwLock<OverlayState>>,
    tip_receiver: Receiver<Tip>,
    /// Whether we've positioned the window on the target monitor
    positioned: bool,
    /// Cached monitor bounds for the selected monitor (x, y, width, height)
    monitor_bounds: Option<(i32, i32, i32, i32)>,
}

impl EguiOverlay for OverlayApp {
    fn gui_run(
        &mut self,
        egui_ctx: &egui::Context,
        _default_gfx_backend: &mut ThreeDBackend,
        glfw_backend: &mut GlfwBackend,
    ) {
        // Position window on the selected monitor and enable passthrough (only on first frame)
        if !self.positioned {
            self.positioned = true;

            // Enable mouse passthrough so clicks go through to the game
            glfw_backend.set_passthrough(true);
            info!("Mouse passthrough enabled");

            let monitor_index = self.state.read().config.monitor_index;

            if let Some(target_index) = monitor_index {
                // First, collect monitor info from GLFW
                let monitor_data: Option<(i32, i32, i32, i32, String)> =
                    glfw_backend.window.glfw.with_connected_monitors(|_, monitors| {
                        monitors.get(target_index).map(|monitor| {
                            let (x, y, w, h) = monitor.get_workarea();
                            let name = monitor.get_name().unwrap_or_else(|| "Unknown".to_string());
                            (x, y, w, h, name)
                        })
                    });

                // Then, apply the positioning outside the closure
                if let Some((x, y, w, h, name)) = monitor_data {
                    self.monitor_bounds = Some((x, y, w, h));

                    // Position and size the window to cover the selected monitor
                    glfw_backend.window.set_pos(x, y);
                    glfw_backend.window.set_size(w, h);

                    info!(
                        "Overlay positioned on monitor {}: {} at ({}, {}) size {}x{}",
                        target_index, name, x, y, w, h
                    );
                } else {
                    // Get monitor count for error message
                    let count = glfw_backend
                        .window
                        .glfw
                        .with_connected_monitors(|_, m| m.len());
                    info!(
                        "Monitor index {} not found, {} monitors available",
                        target_index, count
                    );
                }
            }
        }

        // Process incoming tips
        while let Ok(tip) = self.tip_receiver.try_recv() {
            let mut state = self.state.write();
            let display_tip = DisplayTip::new(tip, state.config.default_duration_ms);
            state.tips.push(display_tip);

            // Limit number of tips
            let max_tips = state.config.max_tips;
            if state.tips.len() > max_tips {
                state.tips.remove(0);
            }
        }

        // Remove expired tips
        {
            let mut state = self.state.write();
            state.tips.retain(|t| !t.is_expired());
        }

        // Get state for rendering
        let state = self.state.read();

        if !state.config.enabled || state.tips.is_empty() {
            // Request repaint to check for new tips
            egui_ctx.request_repaint_after(Duration::from_millis(100));
            return;
        }

        // Determine anchor alignment
        let anchor = match state.config.anchor {
            OverlayAnchor::TopLeft => Align2::LEFT_TOP,
            OverlayAnchor::TopRight => Align2::RIGHT_TOP,
            OverlayAnchor::BottomLeft => Align2::LEFT_BOTTOM,
            OverlayAnchor::BottomRight => Align2::RIGHT_BOTTOM,
        };

        let offset = Vec2::new(
            state.config.offset.0 as f32,
            state.config.offset.1 as f32,
        );

        // Draw tips window
        egui::Area::new(egui::Id::new("tips_overlay"))
            .anchor(anchor, offset)
            .show(egui_ctx, |ui| {
                egui::Frame::none()
                    .fill(Color32::TRANSPARENT)
                    .show(ui, |ui| {
                        ui.set_max_width(350.0);

                        for display_tip in state.tips.iter() {
                            let style = get_style_for_priority(display_tip.tip.priority, &state.styles);
                            let opacity = calculate_opacity(display_tip, state.config.opacity);

                            draw_tip(ui, display_tip, style, opacity);
                            ui.add_space(8.0);
                        }
                    });
            });

        // Request continuous repaints while we have tips
        egui_ctx.request_repaint_after(Duration::from_millis(50));
    }
}

/// Get the appropriate style based on priority
fn get_style_for_priority(priority: u32, styles: &PriorityStyles) -> &TipStyle {
    match priority {
        0..=25 => &styles.low,
        26..=50 => &styles.medium,
        51..=75 => &styles.high,
        _ => &styles.critical,
    }
}

/// Calculate opacity with fade-in/fade-out effects
fn calculate_opacity(tip: &DisplayTip, base_opacity: f32) -> f32 {
    let age = tip.age_secs();

    // Fade in during first 0.3 seconds
    let fade_in = (age / 0.3).min(1.0);

    // Fade out during last 0.5 seconds before expiry
    let fade_out = if let Some(expires_at) = tip.expires_at {
        let remaining = expires_at.saturating_duration_since(Instant::now()).as_secs_f32();
        if remaining < 0.5 {
            remaining / 0.5
        } else {
            1.0
        }
    } else {
        1.0
    };

    base_opacity * fade_in * fade_out
}

/// Draw a single tip
fn draw_tip(ui: &mut egui::Ui, display_tip: &DisplayTip, style: &TipStyle, opacity: f32) {
    let bg_color = Color32::from_rgba_unmultiplied(
        (style.background[0] * 255.0) as u8,
        (style.background[1] * 255.0) as u8,
        (style.background[2] * 255.0) as u8,
        (style.background[3] * opacity * 255.0) as u8,
    );

    let text_color = Color32::from_rgba_unmultiplied(
        (style.text_color[0] * 255.0) as u8,
        (style.text_color[1] * 255.0) as u8,
        (style.text_color[2] * 255.0) as u8,
        (style.text_color[3] * opacity * 255.0) as u8,
    );

    egui::Frame::none()
        .fill(bg_color)
        .rounding(Rounding::same(style.corner_radius))
        .inner_margin(style.padding)
        .show(ui, |ui| {
            ui.label(
                RichText::new(&display_tip.tip.message)
                    .color(text_color)
                    .font(FontId::proportional(14.0)),
            );
        });
}
