//! Dashboard view state management

use std::time::Instant;
use crate::storage::profiles::GameProfile;

/// Current view in the dashboard
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DashboardView {
    #[default]
    Home,
    Capture,
    Overlay,
    Profiles,
    Settings,
}

impl DashboardView {
    /// Get the display name for this view
    pub fn name(&self) -> &'static str {
        match self {
            DashboardView::Home => "Home",
            DashboardView::Capture => "Capture",
            DashboardView::Overlay => "Overlay",
            DashboardView::Profiles => "Profiles",
            DashboardView::Settings => "Settings",
        }
    }

    /// Get the icon character for this view
    pub fn icon(&self) -> &'static str {
        match self {
            DashboardView::Home => "H",
            DashboardView::Capture => "C",
            DashboardView::Overlay => "O",
            DashboardView::Profiles => "P",
            DashboardView::Settings => "S",
        }
    }
}

/// Overall dashboard state
#[derive(Debug)]
pub struct DashboardState {
    /// Current active view
    pub current_view: DashboardView,
    /// Home view state
    pub home: HomeViewState,
    /// Capture view state
    pub capture: CaptureViewState,
    /// Overlay view state
    pub overlay: OverlayViewState,
    /// Profiles view state
    pub profiles: ProfilesViewState,
    /// Settings view state
    pub settings: SettingsViewState,
}

impl Default for DashboardState {
    fn default() -> Self {
        Self {
            current_view: DashboardView::Home,
            home: HomeViewState::default(),
            capture: CaptureViewState::default(),
            overlay: OverlayViewState::default(),
            profiles: ProfilesViewState::default(),
            settings: SettingsViewState::default(),
        }
    }
}

/// Home view state
#[derive(Debug, Default)]
pub struct HomeViewState {
    /// Quick actions expanded
    pub quick_actions_expanded: bool,
}

/// Capture view state
#[derive(Debug)]
pub struct CaptureViewState {
    /// Available windows for capture
    pub available_windows: Vec<String>,
    /// Available monitors for capture
    pub available_monitors: Vec<String>,
    /// Currently selected target type (0 = window, 1 = monitor)
    pub target_type: usize,
    /// Selected window index
    pub selected_window: Option<usize>,
    /// Selected monitor index
    pub selected_monitor: Option<usize>,
    /// Search/filter text
    pub search_query: String,
    /// Preview enabled
    pub preview_enabled: bool,
    /// Last refresh time
    pub last_refresh: Option<Instant>,
}

impl Default for CaptureViewState {
    fn default() -> Self {
        Self {
            available_windows: Vec::new(),
            available_monitors: Vec::new(),
            target_type: 0,
            selected_window: None,
            selected_monitor: None,
            search_query: String::new(),
            preview_enabled: false,
            last_refresh: None,
        }
    }
}

/// Overlay view state
#[derive(Debug, Default)]
pub struct OverlayViewState {
    /// Preview tip text
    pub preview_tip_text: String,
    /// Preview tip priority
    pub preview_tip_priority: u32,
    /// Show tip preview
    pub show_preview: bool,
}

/// Profiles view state
#[derive(Debug, Default)]
pub struct ProfilesViewState {
    /// Search query for filtering profiles
    pub search_query: String,
    /// Currently selected profile ID
    pub selected_profile_id: Option<String>,
    /// Profile being edited (cloned for editing)
    pub editing_profile: Option<GameProfile>,
    /// Show create dialog
    pub show_create_dialog: bool,
    /// Show delete confirmation
    pub show_delete_confirm: bool,
    /// New profile name (for create dialog)
    pub new_profile_name: String,
    /// New profile executable
    pub new_profile_executable: String,
}

/// Settings view state
#[derive(Debug, Default)]
pub struct SettingsViewState {
    /// Currently expanded section
    pub expanded_section: Option<SettingsSection>,
    /// Unsaved changes flag
    pub has_unsaved_changes: bool,
}

/// Settings sections
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsSection {
    General,
    Capture,
    Overlay,
    Performance,
}
