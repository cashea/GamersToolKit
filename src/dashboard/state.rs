//! Dashboard view state management

use std::collections::HashMap;
use std::time::Instant;
use strsim::normalized_levenshtein;
use crate::config::DashboardViewSetting;
use crate::storage::profiles::{GameProfile, LabeledRegion, OcrRegion};

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

    /// Convert to persistable setting
    pub fn to_setting(&self) -> DashboardViewSetting {
        match self {
            DashboardView::Home => DashboardViewSetting::Home,
            DashboardView::Capture => DashboardViewSetting::Capture,
            DashboardView::Overlay => DashboardViewSetting::Overlay,
            DashboardView::Vision => DashboardViewSetting::Vision,
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

    // Region labeling state
    /// Search text for finding regions
    pub region_search_text: String,
    /// Filtered OCR results matching search text
    pub matching_regions: Vec<usize>,
    /// Currently selected region index (for labeling)
    pub selected_region_index: Option<usize>,
    /// Label text being entered for selected region
    pub pending_label: String,
    /// Saved labeled regions
    pub labeled_regions: Vec<LabeledRegion>,
    /// Live values for labeled regions (updated from OCR results)
    pub labeled_regions_live: Vec<LabeledRegionLive>,
    /// Index of labeled region being edited (None = creating new)
    pub editing_labeled_region: Option<usize>,
    /// Flag indicating labels have been modified and need saving
    pub labels_dirty: bool,
    /// Flag to trigger immediate profile save
    pub pending_profile_save: bool,

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
            // Region labeling defaults
            region_search_text: String::new(),
            matching_regions: Vec::new(),
            selected_region_index: None,
            pending_label: String::new(),
            labeled_regions: Vec::new(),
            labeled_regions_live: Vec::new(),
            editing_labeled_region: None,
            labels_dirty: false,
            pending_profile_save: false,
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

/// Live value for a labeled region, updated from OCR results
#[derive(Debug, Clone, Default)]
pub struct LabeledRegionLive {
    /// Current detected text (None if no match found)
    pub current_text: Option<String>,
    /// Current confidence score
    pub current_confidence: Option<f32>,
    /// Index of the matched OCR result
    pub matched_ocr_index: Option<usize>,
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

impl VisionViewState {
    /// Update labeled regions with current OCR results using bounds overlap and fuzzy text matching
    /// Uses match_threshold for fuzzy text similarity when bounds overlap is insufficient
    /// Returns true if any labels were updated
    pub fn update_labels_from_ocr(&mut self) -> bool {
        if self.labeled_regions.is_empty() || self.last_ocr_results.is_empty() {
            // Clear live values if no OCR results
            self.labeled_regions_live.clear();
            return false;
        }

        // Ensure live values vec matches labeled_regions length
        self.labeled_regions_live.resize_with(
            self.labeled_regions.len(),
            LabeledRegionLive::default,
        );

        let mut any_updated = false;
        let threshold = self.match_threshold;

        for (label_idx, labeled) in self.labeled_regions.iter().enumerate() {
            let live = &mut self.labeled_regions_live[label_idx];
            let old_text = live.current_text.clone();

            // Find best matching OCR result using combined scoring:
            // 1. High bounds overlap (>50%) with any text = good match
            // 2. Moderate bounds overlap (>20%) with similar text = good match
            // 3. Any bounds overlap with very similar text = acceptable match
            let mut best_match: Option<(usize, f32, f32)> = None; // (index, combined_score, text_similarity)

            for (ocr_idx, ocr_result) in self.last_ocr_results.iter().enumerate() {
                let bounds_overlap = calculate_bounds_overlap(labeled.bounds, ocr_result.bounds);
                let text_similarity = fuzzy_text_similarity(&labeled.matched_text, &ocr_result.text);

                // Calculate combined score:
                // - High bounds overlap is most important (position-based matching)
                // - Text similarity acts as a tiebreaker and validation
                let combined_score = if bounds_overlap > 0.5 {
                    // Strong position match - accept with bonus for text similarity
                    bounds_overlap + (text_similarity * 0.3)
                } else if bounds_overlap > 0.2 && text_similarity >= threshold {
                    // Moderate position match with good text match
                    bounds_overlap + (text_similarity * 0.5)
                } else if bounds_overlap > 0.1 && text_similarity >= threshold * 1.1 {
                    // Weak position match needs very good text match
                    (bounds_overlap * 0.5) + (text_similarity * 0.5)
                } else {
                    0.0 // No match
                };

                if combined_score > 0.0 {
                    if best_match.map_or(true, |(_, best_score, _)| combined_score > best_score) {
                        best_match = Some((ocr_idx, combined_score, text_similarity));
                    }
                }
            }

            if let Some((ocr_idx, _, text_sim)) = best_match {
                let ocr_result = &self.last_ocr_results[ocr_idx];
                live.current_text = Some(ocr_result.text.clone());
                // Store the text similarity as the "confidence" since OCR doesn't provide it
                live.current_confidence = Some(text_sim);
                live.matched_ocr_index = Some(ocr_idx);

                if old_text.as_ref() != Some(&ocr_result.text) {
                    any_updated = true;
                }
            } else {
                // No match found - clear live values
                if live.current_text.is_some() {
                    any_updated = true;
                }
                live.current_text = None;
                live.current_confidence = None;
                live.matched_ocr_index = None;
            }
        }

        any_updated
    }
}

/// Calculate overlap ratio between two bounding boxes
/// Returns a value from 0.0 (no overlap) to 1.0 (perfect overlap)
fn calculate_bounds_overlap(a: (u32, u32, u32, u32), b: (u32, u32, u32, u32)) -> f32 {
    let (ax, ay, aw, ah) = a;
    let (bx, by, bw, bh) = b;

    // Calculate intersection
    let x1 = ax.max(bx);
    let y1 = ay.max(by);
    let x2 = (ax + aw).min(bx + bw);
    let y2 = (ay + ah).min(by + bh);

    if x1 >= x2 || y1 >= y2 {
        return 0.0; // No overlap
    }

    let intersection_area = (x2 - x1) * (y2 - y1);
    let a_area = aw * ah;
    let b_area = bw * bh;

    // Use IoU (Intersection over Union) for overlap score
    let union_area = a_area + b_area - intersection_area;
    if union_area == 0 {
        return 0.0;
    }

    intersection_area as f32 / union_area as f32
}

/// Calculate fuzzy text similarity between two strings
/// Returns a value from 0.0 (completely different) to 1.0 (identical)
/// Uses normalized Levenshtein distance for robust OCR error tolerance
pub fn fuzzy_text_similarity(a: &str, b: &str) -> f32 {
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }

    // Normalize: lowercase and trim whitespace
    let a_normalized = a.to_lowercase();
    let b_normalized = b.to_lowercase();

    // Calculate base similarity
    let base_similarity = normalized_levenshtein(&a_normalized, &b_normalized) as f32;

    // Also compare with punctuation removed (OCR often drops periods, commas, etc.)
    // This helps match "4.5M" with "45M" or "1,000" with "1000"
    let a_no_punct: String = a_normalized.chars().filter(|c| c.is_alphanumeric()).collect();
    let b_no_punct: String = b_normalized.chars().filter(|c| c.is_alphanumeric()).collect();

    let punct_similarity = if !a_no_punct.is_empty() && !b_no_punct.is_empty() {
        normalized_levenshtein(&a_no_punct, &b_no_punct) as f32
    } else {
        0.0
    };

    // Return the better of the two scores
    base_similarity.max(punct_similarity)
}

// LabeledRegion is now imported from crate::storage::profiles

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
