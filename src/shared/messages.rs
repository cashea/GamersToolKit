//! Message types for communication between dashboard and overlay

use crate::analysis::Tip;
use crate::overlay::OverlayConfig;

/// Messages sent from dashboard to overlay
#[derive(Debug, Clone)]
pub enum DashboardToOverlay {
    /// Update the overlay configuration
    UpdateConfig(OverlayConfig),
    /// Show a tip on the overlay
    ShowTip(Tip),
    /// Set overlay visibility
    SetVisible(bool),
    /// Clear all tips
    ClearTips,
    /// Request the overlay to shutdown
    Shutdown,
}

/// Messages sent from overlay to dashboard
#[derive(Debug, Clone)]
pub enum OverlayToDashboard {
    /// Status update from the overlay
    StatusUpdate(OverlayStatus),
    /// Error occurred in the overlay
    Error(String),
    /// Overlay has started
    Started,
    /// Overlay has stopped
    Stopped,
}

/// Current status of the overlay
#[derive(Debug, Clone, Default)]
pub struct OverlayStatus {
    /// Whether the overlay is visible
    pub visible: bool,
    /// Number of tips currently displayed
    pub tips_count: usize,
    /// Whether mouse passthrough is enabled
    pub passthrough_enabled: bool,
    /// Current monitor index
    pub monitor_index: Option<usize>,
}
