//! Shared application state between dashboard and overlay

use crate::config::AppConfig;
use crate::overlay::OverlayConfig;
use crate::capture::CaptureConfig;
use crate::storage::profiles::GameProfile;

/// Central shared state between dashboard and overlay
#[derive(Debug, Clone)]
pub struct SharedAppState {
    /// Application configuration
    pub config: AppConfig,
    /// Overlay-specific configuration
    pub overlay_config: OverlayConfig,
    /// Capture configuration
    pub capture_config: CaptureConfig,
    /// Loaded game profiles
    pub profiles: Vec<GameProfile>,
    /// Currently active profile ID
    pub active_profile_id: Option<String>,
    /// Runtime state (not persisted)
    pub runtime: RuntimeState,
}

impl Default for SharedAppState {
    fn default() -> Self {
        Self {
            config: AppConfig::default(),
            overlay_config: OverlayConfig::default(),
            capture_config: CaptureConfig::default(),
            profiles: Vec::new(),
            active_profile_id: None,
            runtime: RuntimeState::default(),
        }
    }
}

impl SharedAppState {
    /// Create a new shared state with the given configuration
    pub fn new(config: AppConfig) -> Self {
        // Convert config anchor to overlay anchor
        let anchor = match config.overlay.anchor {
            crate::config::OverlayAnchor::TopLeft => crate::overlay::OverlayAnchor::TopLeft,
            crate::config::OverlayAnchor::TopRight => crate::overlay::OverlayAnchor::TopRight,
            crate::config::OverlayAnchor::BottomLeft => crate::overlay::OverlayAnchor::BottomLeft,
            crate::config::OverlayAnchor::BottomRight => crate::overlay::OverlayAnchor::BottomRight,
        };

        let overlay_config = OverlayConfig {
            opacity: config.overlay.opacity,
            enabled: config.overlay.enabled,
            offset: config.overlay.offset,
            anchor,
            max_tips: config.overlay.max_tips,
            default_duration_ms: config.overlay.default_duration_ms,
            max_width: config.overlay.max_width,
            monitor_index: config.overlay.monitor_index,
            click_through: config.overlay.click_through,
            visible: true,
        };

        let capture_config = CaptureConfig {
            target: if let Some(ref window) = config.capture.target_window {
                crate::capture::CaptureTarget::Window(window.clone())
            } else {
                crate::capture::CaptureTarget::PrimaryMonitor
            },
            max_fps: config.capture.max_fps,
            capture_cursor: config.capture.capture_cursor,
            draw_border: config.capture.draw_border,
        };

        Self {
            config,
            overlay_config,
            capture_config,
            profiles: Vec::new(),
            active_profile_id: None,
            runtime: RuntimeState::default(),
        }
    }

    /// Get the active profile if one is selected
    pub fn active_profile(&self) -> Option<&GameProfile> {
        self.active_profile_id.as_ref().and_then(|id| {
            self.profiles.iter().find(|p| &p.id == id)
        })
    }

    /// Set the active profile by ID
    pub fn set_active_profile(&mut self, profile_id: Option<String>) {
        self.active_profile_id = profile_id;
    }

    /// Add a profile to the list
    pub fn add_profile(&mut self, profile: GameProfile) {
        // Remove existing profile with same ID if it exists
        self.profiles.retain(|p| p.id != profile.id);
        self.profiles.push(profile);
    }

    /// Remove a profile by ID
    pub fn remove_profile(&mut self, profile_id: &str) -> Option<GameProfile> {
        if let Some(pos) = self.profiles.iter().position(|p| p.id == profile_id) {
            // Clear active profile if we're removing it
            if self.active_profile_id.as_deref() == Some(profile_id) {
                self.active_profile_id = None;
            }
            Some(self.profiles.remove(pos))
        } else {
            None
        }
    }
}

/// Command to control capture from UI
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureCommand {
    /// Start capture
    Start,
    /// Stop capture
    Stop,
}

/// Command to control overlay from UI
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayCommand {
    /// Start overlay
    Start,
    /// Stop overlay
    Stop,
    /// Toggle visibility
    ToggleVisibility,
}

/// Runtime state that is not persisted
#[derive(Debug, Clone, Default)]
pub struct RuntimeState {
    /// Whether screen capture is currently active
    pub is_capturing: bool,
    /// Whether the overlay is running
    pub is_overlay_running: bool,
    /// Whether the overlay is currently visible
    pub overlay_visible: bool,
    /// Current capture target description
    pub current_capture_target: Option<String>,
    /// Last error message (if any)
    pub last_error: Option<String>,
    /// Current FPS of capture
    pub capture_fps: f32,
    /// Number of tips currently displayed
    pub tips_displayed: usize,
    /// Pending capture command from UI
    pub capture_command: Option<CaptureCommand>,
    /// Pending overlay command from UI
    pub overlay_command: Option<OverlayCommand>,
    /// Request to send a test tip
    pub send_test_tip: bool,
}

impl RuntimeState {
    /// Clear any error state
    pub fn clear_error(&mut self) {
        self.last_error = None;
    }

    /// Set an error message
    pub fn set_error(&mut self, error: impl Into<String>) {
        self.last_error = Some(error.into());
    }
}
