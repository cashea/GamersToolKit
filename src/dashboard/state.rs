//! Dashboard view state management

use std::collections::HashMap;
use std::time::Instant;
use crate::config::DashboardViewSetting;
use crate::storage::profiles::{GameProfile, OcrRegion};

/// OCR result granularity - word-level or line-level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OcrGranularity {
    /// Individual words with their bounding boxes
    #[default]
    Word,
    /// Full lines with their bounding boxes (words joined)
    Line,
}

/// Current view in the dashboard
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DashboardView {
    #[default]
    Home,
    Capture,
    Overlay,
    Vision,
    Screens,
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
            DashboardView::Vision => "Vision",
            DashboardView::Screens => "Screens",
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
            DashboardView::Vision => "V",
            DashboardView::Screens => "S",
            DashboardView::Profiles => "P",
            DashboardView::Settings => "G", // "Gear" for settings
        }
    }

    /// Convert to persistable setting
    pub fn to_setting(&self) -> DashboardViewSetting {
        match self {
            DashboardView::Home => DashboardViewSetting::Home,
            DashboardView::Capture => DashboardViewSetting::Capture,
            DashboardView::Overlay => DashboardViewSetting::Overlay,
            DashboardView::Vision => DashboardViewSetting::Vision,
            DashboardView::Screens => DashboardViewSetting::Vision, // Map to Vision for now
            DashboardView::Profiles => DashboardViewSetting::Profiles,
            DashboardView::Settings => DashboardViewSetting::Settings,
        }
    }

    /// Convert from persistable setting
    pub fn from_setting(setting: DashboardViewSetting) -> Self {
        match setting {
            DashboardViewSetting::Home => DashboardView::Home,
            DashboardViewSetting::Capture => DashboardView::Capture,
            DashboardViewSetting::Overlay => DashboardView::Overlay,
            DashboardViewSetting::Vision => DashboardView::Vision,
            DashboardViewSetting::Profiles => DashboardView::Profiles,
            DashboardViewSetting::Settings => DashboardView::Settings,
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
    /// Vision view state
    pub vision: VisionViewState,
    /// Screens view state
    pub screens: ScreensViewState,
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
            vision: VisionViewState::default(),
            screens: ScreensViewState::default(),
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
    /// Cached preview texture handle
    pub preview_texture: Option<egui::TextureHandle>,
    /// Last preview frame dimensions (to detect size changes)
    pub preview_frame_size: Option<(u32, u32)>,
}

impl std::fmt::Debug for CaptureViewState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CaptureViewState")
            .field("available_windows", &self.available_windows)
            .field("available_monitors", &self.available_monitors)
            .field("target_type", &self.target_type)
            .field("selected_window", &self.selected_window)
            .field("selected_monitor", &self.selected_monitor)
            .field("search_query", &self.search_query)
            .field("preview_enabled", &self.preview_enabled)
            .field("last_refresh", &self.last_refresh)
            .field("preview_texture", &self.preview_texture.as_ref().map(|_| "<texture>"))
            .field("preview_frame_size", &self.preview_frame_size)
            .finish()
    }
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
            preview_texture: None,
            preview_frame_size: None,
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

/// Vision/OCR view state
pub struct VisionViewState {
    /// Selected OCR backend
    pub selected_backend: crate::vision::OcrBackend,
    /// OCR result granularity (word vs line level)
    pub ocr_granularity: OcrGranularity,
    /// Whether OCR models are ready (PaddleOCR)
    pub models_ready: bool,
    /// Detection model loaded
    pub detection_model_ready: bool,
    /// Recognition model loaded
    pub recognition_model_ready: bool,
    /// OCR engine initialized (PaddleOCR)
    pub ocr_initialized: bool,
    /// Windows OCR initialized
    pub windows_ocr_initialized: bool,
    /// Currently downloading models
    pub is_downloading: bool,
    /// Download progress (0.0 to 1.0)
    pub download_progress: f32,
    /// Currently processing OCR
    pub is_processing: bool,
    /// Pending model download request
    pub pending_download: bool,
    /// Pending OCR init request
    pub pending_init: bool,
    /// Pending OCR run request
    pub pending_ocr_run: bool,
    /// Auto-run OCR on new frames
    pub auto_run_ocr: bool,
    /// Show bounding boxes on preview
    pub show_bounding_boxes: bool,
    /// Match threshold for fuzzy text matching (0.0 - 1.0)
    pub match_threshold: f32,
    /// Image preprocessing settings for OCR
    pub preprocessing: crate::config::OcrPreprocessing,
    /// Last OCR results
    pub last_ocr_results: Vec<OcrResultDisplay>,
    /// Last processing time in ms
    pub last_processing_time_ms: u64,
    /// Last error message
    pub last_error: Option<String>,
    /// Preview texture handle
    pub preview_texture: Option<egui::TextureHandle>,
    /// Preview frame size
    pub preview_frame_size: Option<(u32, u32)>,
    /// Last captured frame data for OCR
    pub last_frame_data: Option<Vec<u8>>,
    /// Last frame width
    pub last_frame_width: u32,
    /// Last frame height
    pub last_frame_height: u32,

    // Zone OCR state
    /// Zone selection state
    pub zone_selection: ZoneSelectionState,
    /// OCR zones defined for current profile
    pub ocr_zones: Vec<OcrRegion>,
    /// Latest OCR results per zone
    pub zone_ocr_results: HashMap<String, ZoneOcrResult>,
    /// Whether to show zone overlays in preview
    pub show_zone_overlays: bool,
    /// Request to enter zone selection mode (triggers overlay mode change)
    pub pending_zone_selection_mode: bool,
    /// Flag indicating zones have been modified and need saving
    pub zones_dirty: bool,
    /// Error message for zone selection (e.g., overlay failed to start)
    pub zone_selection_error: Option<String>,
}

impl std::fmt::Debug for VisionViewState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VisionViewState")
            .field("models_ready", &self.models_ready)
            .field("ocr_initialized", &self.ocr_initialized)
            .field("is_downloading", &self.is_downloading)
            .field("is_processing", &self.is_processing)
            .field("last_ocr_results_count", &self.last_ocr_results.len())
            .field("last_processing_time_ms", &self.last_processing_time_ms)
            .finish()
    }
}

impl Default for VisionViewState {
    fn default() -> Self {
        Self {
            selected_backend: crate::vision::OcrBackend::WindowsOcr, // Default to Windows OCR
            ocr_granularity: OcrGranularity::Word, // Default to word-level
            models_ready: false,
            detection_model_ready: false,
            recognition_model_ready: false,
            ocr_initialized: false,
            windows_ocr_initialized: false,
            is_downloading: false,
            download_progress: 0.0,
            is_processing: false,
            pending_download: false,
            pending_init: false,
            pending_ocr_run: false,
            auto_run_ocr: false,
            show_bounding_boxes: true,
            match_threshold: 0.8,
            preprocessing: crate::config::OcrPreprocessing::default(),
            last_ocr_results: Vec::new(),
            last_processing_time_ms: 0,
            last_error: None,
            preview_texture: None,
            preview_frame_size: None,
            last_frame_data: None,
            last_frame_width: 0,
            last_frame_height: 0,
            // Zone OCR defaults
            zone_selection: ZoneSelectionState::default(),
            ocr_zones: Vec::new(),
            zone_ocr_results: HashMap::new(),
            show_zone_overlays: true,
            pending_zone_selection_mode: false,
            zones_dirty: false,
            zone_selection_error: None,
        }
    }
}

/// OCR result for display
#[derive(Debug, Clone)]
pub struct OcrResultDisplay {
    /// Detected text
    pub text: String,
    /// Bounding box (x, y, width, height)
    pub bounds: (u32, u32, u32, u32),
    /// Confidence score
    pub confidence: f32,
}

/// State for zone selection mode
#[derive(Debug, Clone, Default)]
pub struct ZoneSelectionState {
    /// Whether zone selection mode is active
    pub is_selecting: bool,
    /// Current selection rectangle (normalized 0.0-1.0): (x, y, width, height)
    pub current_selection: Option<(f32, f32, f32, f32)>,
    /// Name being entered for new zone
    pub pending_zone_name: String,
    /// Content type for new zone
    pub pending_content_type: crate::storage::profiles::ContentType,
    /// Whether to show the zone naming dialog
    pub show_naming_dialog: bool,
    /// Zone being edited (None = creating new)
    pub editing_zone_id: Option<String>,
    /// Whether to show the zone settings dialog
    pub show_settings_dialog: bool,
    /// Zone index being configured (for settings dialog)
    pub settings_zone_index: Option<usize>,
    /// Pending preprocessing settings for the zone being edited
    pub pending_preprocessing: crate::config::OcrPreprocessing,
    /// Whether to use custom preprocessing for this zone
    pub pending_use_custom_preprocessing: bool,
    /// Zone index being repositioned (None = creating new zone)
    pub repositioning_zone_index: Option<usize>,
    /// Auto-configure state for a zone
    pub auto_configure: Option<AutoConfigureState>,
}

/// State for auto-configure process
#[derive(Debug, Clone)]
pub struct AutoConfigureState {
    /// Zone index being auto-configured
    pub zone_index: usize,
    /// Current step in the auto-configure process
    pub current_step: AutoConfigureStep,
    /// Current scale being tested (1-4)
    pub current_scale: u32,
    /// Current preprocessing enabled state
    pub current_preprocessing_enabled: bool,
    /// Current grayscale setting
    pub current_grayscale: bool,
    /// Current invert setting
    pub current_invert: bool,
    /// Current contrast value
    pub current_contrast: f32,
    /// Total combinations to try
    pub total_combinations: usize,
    /// Current combination index
    pub current_combination: usize,
    /// Status message to display
    pub status_message: String,
    /// Whether auto-configure completed successfully
    pub success: bool,
    /// Error message if failed
    pub error_message: Option<String>,
    /// Best configuration found so far (preprocessing settings)
    pub best_preprocessing: Option<crate::config::OcrPreprocessing>,
    /// Best confidence score found
    pub best_confidence: f32,
    /// Best text found
    pub best_text: String,
}

/// Steps in the auto-configure process
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutoConfigureStep {
    /// Starting the process
    Starting,
    /// Testing a configuration
    Testing,
    /// Completed (success or failure)
    Completed,
}

/// Zone OCR result for display
#[derive(Debug, Clone)]
pub struct ZoneOcrResult {
    /// Zone ID
    pub zone_id: String,
    /// Zone name
    pub zone_name: String,
    /// Detected text
    pub text: String,
    /// Last update timestamp
    pub last_updated: Instant,
}

/// Action to perform on a profile (for UI-to-app communication)
#[derive(Debug, Clone)]
pub enum ProfileAction {
    /// Activate a profile by ID
    Activate(String),
    /// Deactivate the current profile
    Deactivate,
    /// Create a new profile (includes the profile to save)
    Create(GameProfile),
    /// Delete a profile by ID
    Delete(String),
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
    /// Pending profile action (processed by DashboardApp)
    pub pending_action: Option<ProfileAction>,
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

/// Screen recognition view state
#[derive(Default)]
pub struct ScreensViewState {
    /// Currently selected screen ID in the hierarchy tree
    pub selected_screen_id: Option<String>,
    /// Whether to show the add screen dialog
    pub show_add_dialog: bool,
    /// Whether to show the delete confirmation dialog
    pub show_delete_confirm: bool,
    /// New screen name (for add dialog)
    pub new_screen_name: String,
    /// New screen parent ID (for add dialog)
    pub new_screen_parent_id: Option<String>,
    /// New screen match mode (for add dialog)
    pub new_screen_match_mode: crate::storage::profiles::ScreenMatchMode,
    /// Whether screen recognition is running
    pub recognition_running: bool,
    /// Pending request to capture full screen template
    pub pending_full_capture: bool,
    /// Pending request to capture visual anchor
    pub pending_anchor_capture: Option<String>, // Screen ID to add anchor to
    /// Pending request to capture text anchor
    pub pending_text_anchor_capture: Option<String>, // Screen ID to add anchor to
    /// Screens marked as dirty (need saving)
    pub screens_dirty: bool,
    /// Error message to display
    pub error_message: Option<String>,
    /// Preview texture for current screen template
    pub preview_texture: Option<egui::TextureHandle>,
    /// Pending text anchor waiting for user confirmation: (screen_id, detected_text, bounds)
    pub pending_text_for_anchor: Option<(String, String, (f32, f32, f32, f32))>,
    /// Editable text field for text anchor confirmation dialog
    pub editing_text_anchor_text: String,
    /// Screen ID being dragged for priority reordering
    pub dragging_screen_id: Option<String>,
    /// Drop target screen ID (screen being hovered over during drag)
    pub drop_target_screen_id: Option<String>,
    /// Whether to drop before (true) or after (false) the target
    pub drop_before_target: bool,
}

impl std::fmt::Debug for ScreensViewState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScreensViewState")
            .field("selected_screen_id", &self.selected_screen_id)
            .field("show_add_dialog", &self.show_add_dialog)
            .field("show_delete_confirm", &self.show_delete_confirm)
            .field("new_screen_name", &self.new_screen_name)
            .field("recognition_running", &self.recognition_running)
            .field("screens_dirty", &self.screens_dirty)
            .field("has_preview_texture", &self.preview_texture.is_some())
            .finish()
    }
}
