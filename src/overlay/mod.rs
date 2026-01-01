//! Overlay Presentation Layer
//!
//! Displays tips and alerts using egui_overlay with click passthrough.
//! The overlay is a separate window that doesn't interact with the game.

pub mod widgets;
pub mod zone_selection;

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
use crate::overlay::zone_selection::{ZoneSelectionOverlayState, render_zone_selection};

/// Mode for overlay interaction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OverlayMode {
    /// Normal display mode (tips only, click-through)
    #[default]
    Normal,
    /// Zone selection mode (interactive, captures mouse)
    ZoneSelection,
}

/// Commands sent from dashboard to overlay for zone selection
#[derive(Debug, Clone)]
pub enum ZoneCommand {
    /// Enter zone selection mode with existing zones to display
    EnterSelectionMode {
        existing_zones: Vec<(String, (f32, f32, f32, f32))>,
        /// Capture frame dimensions (width, height) for proper coordinate normalization
        capture_size: Option<(u32, u32)>,
    },
    /// Exit zone selection mode
    ExitSelectionMode,
    /// Update zone display (when zones change in dashboard)
    UpdateZones {
        zones: Vec<(String, (f32, f32, f32, f32))>,
    },
}

/// Results sent from overlay back to dashboard
#[derive(Debug, Clone)]
pub enum ZoneSelectionResult {
    /// User completed a selection
    Completed {
        bounds: (f32, f32, f32, f32),
    },
    /// User cancelled selection
    Cancelled,
}

/// Overlay configuration
#[derive(Debug, Clone, PartialEq)]
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
    /// Whether clicks pass through the overlay to the game beneath
    pub click_through: bool,
    /// Maximum width for tip display in pixels
    pub max_width: f32,
    /// Whether the overlay is currently visible (can be toggled via hotkey)
    pub visible: bool,
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
            click_through: true,    // Allow clicks to pass through by default
            max_width: 350.0,       // Default tip width in pixels
            visible: true,          // Visible by default
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
    /// Current overlay mode
    mode: OverlayMode,
    /// Zone selection state
    zone_selection: ZoneSelectionOverlayState,
}

impl OverlayState {
    fn new(config: OverlayConfig) -> Self {
        Self {
            tips: Vec::new(),
            config,
            styles: PriorityStyles::default(),
            mode: OverlayMode::Normal,
            zone_selection: ZoneSelectionOverlayState::default(),
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
    /// Channel for sending zone commands to overlay
    zone_cmd_sender: Sender<ZoneCommand>,
    zone_cmd_receiver: Receiver<ZoneCommand>,
    /// Channel for receiving zone selection results from overlay
    zone_result_sender: Sender<ZoneSelectionResult>,
    zone_result_receiver: Receiver<ZoneSelectionResult>,
}

impl OverlayManager {
    /// Create a new overlay manager
    pub fn new(config: OverlayConfig) -> Result<Self> {
        let (tip_sender, tip_receiver) = unbounded();
        let (zone_cmd_sender, zone_cmd_receiver) = unbounded();
        let (zone_result_sender, zone_result_receiver) = unbounded();
        Ok(Self {
            state: Arc::new(RwLock::new(OverlayState::new(config))),
            tip_sender,
            tip_receiver,
            zone_cmd_sender,
            zone_cmd_receiver,
            zone_result_sender,
            zone_result_receiver,
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

    /// Set whether clicks pass through the overlay to the game beneath
    ///
    /// When enabled (default), mouse clicks go through to the game.
    /// When disabled, the overlay window captures mouse input (useful for repositioning).
    pub fn set_click_through(&self, enabled: bool) {
        self.state.write().config.click_through = enabled;
    }

    /// Toggle click-through mode
    ///
    /// Returns the new state (true = click-through enabled)
    pub fn toggle_click_through(&self) -> bool {
        let mut state = self.state.write();
        state.config.click_through = !state.config.click_through;
        state.config.click_through
    }

    /// Enter zone selection mode
    ///
    /// Sends a command to the overlay to enter zone selection mode,
    /// displaying existing zones and allowing the user to draw a new one.
    /// The capture_size should be provided when a capture is active to ensure
    /// zone coordinates are properly normalized relative to the captured content.
    pub fn enter_zone_selection_mode(
        &self,
        existing_zones: Vec<(String, (f32, f32, f32, f32))>,
        capture_size: Option<(u32, u32)>,
    ) {
        let _ = self.zone_cmd_sender.send(ZoneCommand::EnterSelectionMode {
            existing_zones,
            capture_size,
        });
    }

    /// Exit zone selection mode
    pub fn exit_zone_selection_mode(&self) {
        let _ = self.zone_cmd_sender.send(ZoneCommand::ExitSelectionMode);
    }

    /// Update the zones displayed on the overlay
    pub fn update_zones(&self, zones: Vec<(String, (f32, f32, f32, f32))>) {
        let _ = self.zone_cmd_sender.send(ZoneCommand::UpdateZones { zones });
    }

    /// Poll for zone selection results (non-blocking)
    pub fn poll_zone_selection_result(&self) -> Option<ZoneSelectionResult> {
        self.zone_result_receiver.try_recv().ok()
    }

    /// Run the overlay event loop (blocking)
    /// This should be called from the main thread
    pub fn run(&self) -> Result<()> {
        info!("Starting overlay...");

        let state = self.state.clone();
        let tip_receiver = self.tip_receiver.clone();
        let zone_cmd_receiver = self.zone_cmd_receiver.clone();
        let zone_result_sender = self.zone_result_sender.clone();
        let config = self.state.read().config.clone();

        // Create the overlay app
        let app = OverlayApp {
            state,
            tip_receiver,
            zone_cmd_receiver,
            zone_result_sender,
            positioned: false,
            monitor_bounds: None,
            current_click_through: config.click_through,
            current_monitor_index: config.monitor_index,
            capture_size: None,
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
    /// Receiver for zone selection commands
    zone_cmd_receiver: Receiver<ZoneCommand>,
    /// Sender for zone selection results
    zone_result_sender: Sender<ZoneSelectionResult>,
    /// Whether we've positioned the window on the target monitor
    positioned: bool,
    /// Cached monitor bounds for the selected monitor (x, y, width, height)
    monitor_bounds: Option<(i32, i32, i32, i32)>,
    /// Current click-through state (tracked for runtime changes)
    current_click_through: bool,
    /// Current monitor index (tracked for runtime changes)
    current_monitor_index: Option<usize>,
    /// Capture frame dimensions for zone coordinate normalization
    capture_size: Option<(u32, u32)>,
}

impl EguiOverlay for OverlayApp {
    fn gui_run(
        &mut self,
        egui_ctx: &egui::Context,
        _default_gfx_backend: &mut ThreeDBackend,
        glfw_backend: &mut GlfwBackend,
    ) {
        // Process zone commands
        while let Ok(cmd) = self.zone_cmd_receiver.try_recv() {
            let mut state = self.state.write();
            match cmd {
                ZoneCommand::EnterSelectionMode { existing_zones, capture_size } => {
                    state.mode = OverlayMode::ZoneSelection;
                    state.zone_selection.existing_zones = existing_zones;
                    state.zone_selection.start_point = None;
                    state.zone_selection.current_point = None;
                    state.zone_selection.completed_selection = None;
                    // Store capture size for coordinate normalization
                    drop(state); // Release lock before modifying self
                    self.capture_size = capture_size;
                    info!("Entered zone selection mode with capture_size: {:?}", capture_size);
                }
                ZoneCommand::ExitSelectionMode => {
                    state.mode = OverlayMode::Normal;
                    state.zone_selection = ZoneSelectionOverlayState::default();
                    drop(state);
                    self.capture_size = None;
                    info!("Exited zone selection mode");
                }
                ZoneCommand::UpdateZones { zones } => {
                    state.zone_selection.existing_zones = zones;
                }
            }
        }

        // Get current mode to determine click-through behavior
        let current_mode = self.state.read().mode;

        // In zone selection mode, disable click-through to capture mouse
        let desired_click_through = match current_mode {
            OverlayMode::Normal => self.state.read().config.click_through,
            OverlayMode::ZoneSelection => false, // Must capture mouse for drawing
        };

        if desired_click_through != self.current_click_through {
            glfw_backend.set_passthrough(desired_click_through);
            self.current_click_through = desired_click_through;
            info!(
                "Click-through {}",
                if desired_click_through { "enabled" } else { "disabled" }
            );
        }

        // Check if monitor index changed at runtime
        let desired_monitor_index = self.state.read().config.monitor_index;
        let monitor_changed = desired_monitor_index != self.current_monitor_index;

        // Position window on the selected monitor (on first frame or when monitor changes)
        if !self.positioned || monitor_changed {
            if !self.positioned {
                // Set initial mouse passthrough based on config
                glfw_backend.set_passthrough(self.current_click_through);
                info!(
                    "Initial click-through: {}",
                    if self.current_click_through { "enabled" } else { "disabled" }
                );
            }

            self.positioned = true;
            self.current_monitor_index = desired_monitor_index;

            if let Some(target_index) = desired_monitor_index {
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

        // Handle zone selection mode
        if current_mode == OverlayMode::ZoneSelection {
            // Use capture frame dimensions for normalization if available,
            // otherwise fall back to monitor bounds
            let screen_size = self.capture_size
                .map(|(w, h)| (w as f32, h as f32))
                .or_else(|| self.monitor_bounds.map(|(_, _, w, h)| (w as f32, h as f32)))
                .unwrap_or((1920.0, 1080.0));

            let result = {
                let mut state = self.state.write();
                render_zone_selection(egui_ctx, &mut state.zone_selection, screen_size)
            };

            // Handle zone selection result
            if let Some(zone_result) = result {
                let _ = self.zone_result_sender.send(zone_result.clone());

                // If completed or cancelled, exit zone selection mode
                match zone_result {
                    ZoneSelectionResult::Completed { .. } | ZoneSelectionResult::Cancelled => {
                        let mut state = self.state.write();
                        state.mode = OverlayMode::Normal;
                        state.zone_selection = ZoneSelectionOverlayState::default();
                    }
                }
            }

            // Request continuous repaints in zone selection mode
            egui_ctx.request_repaint_after(Duration::from_millis(16));
            return;
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

        if !state.config.enabled || !state.config.visible || state.tips.is_empty() {
            // Request repaint to check for new tips or visibility changes
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

        let max_width = state.config.max_width;

        // Draw tips window
        egui::Area::new(egui::Id::new("tips_overlay"))
            .anchor(anchor, offset)
            .show(egui_ctx, |ui| {
                egui::Frame::none()
                    .fill(Color32::TRANSPARENT)
                    .show(ui, |ui| {
                        ui.set_max_width(max_width);

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
