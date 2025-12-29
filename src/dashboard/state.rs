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
    Vision,
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
    /// Vision view state
    pub vision: VisionViewState,
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
    /// Confidence threshold for display
    pub confidence_threshold: f32,
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
            confidence_threshold: 0.5,
            last_ocr_results: Vec::new(),
            last_processing_time_ms: 0,
            last_error: None,
            preview_texture: None,
            preview_frame_size: None,
            last_frame_data: None,
            last_frame_width: 0,
            last_frame_height: 0,
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
