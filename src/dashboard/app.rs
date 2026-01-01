//! Dashboard application entry point

use eframe::egui;
use parking_lot::{Mutex, RwLock};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::sync::mpsc::{self, Sender};

use crate::analysis::Tip;
use crate::capture::{CaptureTarget, ScreenCapture};
use crate::config::WindowState;
use crate::dashboard::components::render_sidebar;
use crate::dashboard::state::{AutoConfigureStep, DashboardState, DashboardView};
use crate::dashboard::theme;
use crate::dashboard::views::{
    render_capture_view, render_home_view, render_overlay_view,
    render_profiles_view, render_settings_view, render_vision_view,
};
use crate::hotkey::HotkeyManager;
use crate::overlay::{OverlayManager, ZoneSelectionResult};
use crate::shared::SharedAppState;
use crate::storage::profiles::{GameProfile, ContentType};
use crate::dashboard::state::ZoneOcrResult;
use crate::vision::{VisionPipeline, ModelManager, ModelType, OcrGranularity};
use std::thread::JoinHandle;

/// The main dashboard application
pub struct DashboardApp {
    /// Shared application state
    shared_state: Arc<RwLock<SharedAppState>>,
    /// Dashboard-specific state
    dashboard_state: DashboardState,
    /// Whether theme has been applied
    theme_applied: bool,
    /// Screen capture manager
    capture_manager: Arc<Mutex<Option<ScreenCapture>>>,
    /// Frame counter for FPS calculation
    frame_counter: FrameCounter,
    /// Overlay manager
    overlay_manager: Option<Arc<OverlayManager>>,
    /// Overlay thread handle
    overlay_handle: Option<JoinHandle<()>>,
    /// Last synced overlay config (for change detection)
    last_synced_overlay_config: Option<crate::overlay::OverlayConfig>,
    /// Global hotkey manager
    hotkey_manager: Option<HotkeyManager>,
    /// Config directory path for saving
    config_dir: Option<PathBuf>,
    /// Last time config was auto-saved
    last_auto_save: Instant,
    /// Whether there are pending changes to save
    pending_save: bool,
    /// Last saved window state (for change detection)
    last_window_state: Option<WindowState>,
    /// Last time window state was saved
    last_window_save: Instant,
    /// Vision pipeline for OCR
    vision_pipeline: Option<VisionPipeline>,
    /// Model manager for downloading OCR models
    model_manager: Option<ModelManager>,
    /// Profiles directory path for saving
    profiles_dir: Option<PathBuf>,
    /// Currently active profile
    active_profile: Option<GameProfile>,
    /// Last time profile labels were auto-saved
    last_profile_save: Instant,
    /// Last synced vision settings (for change detection)
    last_synced_vision: Option<crate::config::VisionSettings>,
    /// Last synced dashboard view (for change detection)
    last_synced_view: Option<DashboardView>,
}

/// Helper for calculating FPS
struct FrameCounter {
    frames_this_second: u32,
    last_fps_update: Instant,
    current_fps: f32,
}

impl Default for FrameCounter {
    fn default() -> Self {
        Self {
            frames_this_second: 0,
            last_fps_update: Instant::now(),
            current_fps: 0.0,
        }
    }
}

impl DashboardApp {
    /// Create a new dashboard application
    pub fn new(shared_state: Arc<RwLock<SharedAppState>>) -> Self {
        // Initialize hotkey manager
        let hotkey_manager = match HotkeyManager::new(shared_state.clone()) {
            Ok(mut manager) => {
                if let Err(e) = manager.register_toggle_hotkey() {
                    tracing::warn!("Failed to register toggle hotkey: {}", e);
                }
                if let Err(e) = manager.register_zone_selection_hotkey() {
                    tracing::warn!("Failed to register zone selection hotkey: {}", e);
                }
                Some(manager)
            }
            Err(e) => {
                tracing::warn!("Failed to create hotkey manager: {}", e);
                None
            }
        };

        // Get config directory for saving
        let config_dir = crate::storage::get_config_dir().ok();

        // Get profiles directory
        let profiles_dir = crate::storage::get_profiles_dir().ok();

        // Initialize model manager
        let model_manager = ModelManager::new().ok();

        // Load persisted settings from shared state config
        let (vision_settings, dashboard_settings) = {
            let state = shared_state.read();
            (state.config.vision.clone(), state.config.dashboard.clone())
        };

        // Load or create profile based on saved active_profile_id
        let (active_profile, initial_zones) = Self::load_profile_by_id(
            &profiles_dir,
            dashboard_settings.active_profile_id.as_deref(),
        );

        let mut dashboard_state = DashboardState::default();

        // Restore persisted dashboard view
        dashboard_state.current_view = DashboardView::from_setting(dashboard_settings.last_view);

        // Restore persisted vision settings
        dashboard_state.vision.selected_backend = vision_settings.backend;
        dashboard_state.vision.ocr_granularity = match vision_settings.granularity {
            crate::vision::OcrGranularity::Word => crate::dashboard::state::OcrGranularity::Word,
            crate::vision::OcrGranularity::Line => crate::dashboard::state::OcrGranularity::Line,
        };
        dashboard_state.vision.match_threshold = vision_settings.match_threshold;
        dashboard_state.vision.show_bounding_boxes = vision_settings.show_bounding_boxes;
        dashboard_state.vision.auto_run_ocr = vision_settings.auto_run_ocr;
        dashboard_state.vision.preprocessing = vision_settings.preprocessing.clone();

        // Load zones from profile into vision state
        dashboard_state.vision.ocr_zones = initial_zones;

        tracing::info!(
            "Restored settings: view={:?}, backend={:?}, granularity={:?}",
            dashboard_state.current_view,
            dashboard_state.vision.selected_backend,
            dashboard_state.vision.ocr_granularity
        );

        Self {
            shared_state,
            dashboard_state,
            theme_applied: false,
            capture_manager: Arc::new(Mutex::new(None)),
            frame_counter: FrameCounter::default(),
            overlay_manager: None,
            overlay_handle: None,
            last_synced_overlay_config: None,
            hotkey_manager,
            config_dir,
            last_auto_save: Instant::now(),
            pending_save: false,
            last_window_state: None,
            last_window_save: Instant::now(),
            vision_pipeline: None,
            model_manager,
            profiles_dir,
            active_profile,
            last_profile_save: Instant::now(),
            last_synced_vision: Some(vision_settings),
            last_synced_view: Some(DashboardView::from_setting(dashboard_settings.last_view)),
        }
    }

    /// Load a profile by ID, or create default if not found
    /// Returns (profile, ocr_zones)
    fn load_profile_by_id(
        profiles_dir: &Option<PathBuf>,
        profile_id: Option<&str>,
    ) -> (Option<GameProfile>, Vec<crate::storage::profiles::OcrRegion>) {
        let Some(ref dir) = profiles_dir else {
            return (None, vec![]);
        };

        // Try to load the requested profile
        let profile_id = profile_id.unwrap_or("default");
        let profile_path = dir.join(format!("{}.json", profile_id));

        if profile_path.exists() {
            match crate::storage::profiles::load_profile(&profile_path) {
                Ok(profile) => {
                    let zones = profile.ocr_regions.clone();
                    tracing::info!(
                        "Loaded profile '{}' with {} zones",
                        profile.name,
                        zones.len()
                    );
                    return (Some(profile), zones);
                }
                Err(e) => {
                    tracing::warn!("Failed to load profile '{}': {}", profile_id, e);
                }
            }
        }

        // Fall back to default profile if requested profile wasn't found
        if profile_id != "default" {
            let default_path = dir.join("default.json");
            if default_path.exists() {
                match crate::storage::profiles::load_profile(&default_path) {
                    Ok(profile) => {
                        let zones = profile.ocr_regions.clone();
                        tracing::info!(
                            "Loaded fallback default profile with {} zones",
                            zones.len()
                        );
                        return (Some(profile), zones);
                    }
                    Err(e) => {
                        tracing::warn!("Failed to load default profile: {}", e);
                    }
                }
            }
        }

        // Create a new default profile
        let profile = GameProfile {
            id: "default".to_string(),
            name: "Default Profile".to_string(),
            executables: vec![],
            version: "1.0.0".to_string(),
            ocr_regions: vec![],
            templates: vec![],
            rules: vec![],
            labeled_regions: vec![],
        };

        let default_path = dir.join("default.json");
        if let Err(e) = crate::storage::profiles::save_profile(&profile, &default_path) {
            tracing::warn!("Failed to save default profile: {}", e);
        } else {
            tracing::info!("Created new default profile");
        }

        (Some(profile), vec![])
    }

    /// Activate a profile by ID
    /// Saves current zones to old profile, loads new profile's zones
    fn activate_profile(&mut self, profile_id: &str) {
        // First, save current zones to the currently active profile (if any)
        self.save_current_zones_to_profile();

        // Find the profile to activate
        let profile_to_activate = {
            let state = self.shared_state.read();
            state.profiles.iter().find(|p| p.id == profile_id).cloned()
        };

        let Some(profile) = profile_to_activate else {
            tracing::warn!("Cannot activate profile '{}': not found", profile_id);
            return;
        };

        // Load zones from the new profile
        let zones = profile.ocr_regions.clone();
        let zone_count = zones.len();
        let profile_name = profile.name.clone();
        let profile_id_owned = profile.id.clone();

        // Update dashboard state with new zones
        self.dashboard_state.vision.ocr_zones = zones;
        self.dashboard_state.vision.zone_ocr_results.clear();
        self.dashboard_state.vision.zones_dirty = false;

        // Update active profile reference
        self.active_profile = Some(profile);

        // Update shared state
        {
            let mut state = self.shared_state.write();
            state.active_profile_id = Some(profile_id_owned.clone());
            state.config.dashboard.active_profile_id = Some(profile_id_owned);
        }

        // Mark config for save
        self.pending_save = true;
        self.last_auto_save = Instant::now();

        tracing::info!(
            "Activated profile '{}' with {} zones",
            profile_name,
            zone_count
        );
    }

    /// Deactivate the current profile
    /// Saves current zones and clears active profile
    fn deactivate_profile(&mut self) {
        // First, save current zones to the currently active profile
        self.save_current_zones_to_profile();

        let old_profile_name = self.active_profile.as_ref().map(|p| p.name.clone());

        // Clear active profile
        self.active_profile = None;

        // Update shared state
        {
            let mut state = self.shared_state.write();
            state.active_profile_id = None;
            state.config.dashboard.active_profile_id = None;
        }

        // Clear zones from vision state
        self.dashboard_state.vision.ocr_zones.clear();
        self.dashboard_state.vision.zone_ocr_results.clear();
        self.dashboard_state.vision.zones_dirty = false;

        // Mark config for save
        self.pending_save = true;
        self.last_auto_save = Instant::now();

        if let Some(name) = old_profile_name {
            tracing::info!("Deactivated profile '{}'", name);
        }
    }

    /// Save current zones to the active profile (helper method)
    fn save_current_zones_to_profile(&mut self) {
        if let (Some(ref mut profile), Some(ref profiles_dir)) =
            (&mut self.active_profile, &self.profiles_dir)
        {
            // Update profile with current zones
            profile.ocr_regions = self.dashboard_state.vision.ocr_zones.clone();

            // Save to disk
            let profile_path = profiles_dir.join(format!("{}.json", profile.id));
            if let Err(e) = crate::storage::profiles::save_profile(profile, &profile_path) {
                tracing::error!("Failed to save zones to profile '{}': {}", profile.name, e);
            } else {
                tracing::debug!(
                    "Saved {} zones to profile '{}' before switching",
                    profile.ocr_regions.len(),
                    profile.name
                );
            }
        }
    }

    /// Start screen capture
    pub fn start_capture(&mut self) -> Result<(), String> {
        let config = {
            let state = self.shared_state.read();
            state.capture_config.clone()
        };

        let target_name = match &config.target {
            CaptureTarget::Window(name) => name.clone(),
            CaptureTarget::PrimaryMonitor => "Primary Monitor".to_string(),
            CaptureTarget::MonitorIndex(i) => format!("Monitor {}", i),
        };

        match ScreenCapture::new(config) {
            Ok(mut capture) => {
                if let Err(e) = capture.start() {
                    return Err(format!("Failed to start capture: {}", e));
                }
                *self.capture_manager.lock() = Some(capture);

                let mut state = self.shared_state.write();
                state.runtime.is_capturing = true;
                state.runtime.current_capture_target = Some(target_name);
                state.runtime.clear_error();
                self.frame_counter = FrameCounter::default();
                Ok(())
            }
            Err(e) => Err(format!("Failed to create capture: {}", e)),
        }
    }

    /// Stop screen capture
    pub fn stop_capture(&mut self) {
        if let Some(mut capture) = self.capture_manager.lock().take() {
            let _ = capture.stop();
        }

        let mut state = self.shared_state.write();
        state.runtime.is_capturing = false;
        state.runtime.capture_fps = 0.0;
    }

    /// Update capture FPS by polling for frames
    fn update_capture_stats(&mut self) {
        let mut capture_guard = self.capture_manager.lock();
        if let Some(ref capture) = *capture_guard {
            // Try to get frames without blocking to calculate FPS
            while capture.try_next_frame().is_some() {
                self.frame_counter.frames_this_second += 1;
            }

            // Update FPS every second
            let elapsed = self.frame_counter.last_fps_update.elapsed();
            if elapsed.as_secs_f32() >= 1.0 {
                self.frame_counter.current_fps =
                    self.frame_counter.frames_this_second as f32 / elapsed.as_secs_f32();
                self.frame_counter.frames_this_second = 0;
                self.frame_counter.last_fps_update = Instant::now();

                // Update shared state
                let mut state = self.shared_state.write();
                state.runtime.capture_fps = self.frame_counter.current_fps;
            }

            // Check if capture is still running
            if !capture.is_running() {
                drop(capture_guard);
                self.stop_capture();
            }
        }
    }

    /// Check if capture is running
    pub fn is_capturing(&self) -> bool {
        self.capture_manager.lock().as_ref().map(|c| c.is_running()).unwrap_or(false)
    }

    /// Get the capture manager for external use
    pub fn capture_manager(&self) -> Arc<Mutex<Option<ScreenCapture>>> {
        self.capture_manager.clone()
    }

    /// Start the overlay
    pub fn start_overlay(&mut self) -> Result<(), String> {
        if self.overlay_manager.is_some() {
            return Ok(()); // Already running
        }

        let overlay_config = self.shared_state.read().overlay_config.clone();

        match OverlayManager::new(overlay_config) {
            Ok(manager) => {
                let manager = Arc::new(manager);
                self.overlay_manager = Some(manager.clone());

                // Start overlay in a background thread
                let handle = std::thread::spawn(move || {
                    if let Err(e) = manager.run() {
                        tracing::error!("Overlay error: {}", e);
                    }
                });

                self.overlay_handle = Some(handle);

                // Update runtime state
                let mut state = self.shared_state.write();
                state.runtime.is_overlay_running = true;
                state.runtime.overlay_visible = true;

                Ok(())
            }
            Err(e) => Err(format!("Failed to create overlay: {}", e)),
        }
    }

    /// Stop the overlay
    pub fn stop_overlay(&mut self) {
        self.overlay_manager = None;

        // The overlay thread will stop when the manager is dropped
        if let Some(handle) = self.overlay_handle.take() {
            let _ = handle.join();
        }

        let mut state = self.shared_state.write();
        state.runtime.is_overlay_running = false;
        state.runtime.overlay_visible = false;
    }

    /// Check if overlay is running
    pub fn is_overlay_running(&self) -> bool {
        self.overlay_handle
            .as_ref()
            .map(|h| !h.is_finished())
            .unwrap_or(false)
    }

    /// Create eframe options for the dashboard window
    pub fn options() -> eframe::NativeOptions {
        // Try to load saved window state
        let window_state = crate::storage::get_config_dir()
            .ok()
            .and_then(|dir| crate::config::load_window_state(&dir.join("window_state.toml")));

        let mut viewport = egui::ViewportBuilder::default()
            .with_min_inner_size([800.0, 500.0])
            .with_title("GamersToolKit Dashboard");

        // Apply saved window state or defaults
        if let Some(state) = window_state {
            if let Some((w, h)) = state.size {
                viewport = viewport.with_inner_size([w, h]);
            } else {
                viewport = viewport.with_inner_size([1100.0, 700.0]);
            }
            if let Some((x, y)) = state.position {
                viewport = viewport.with_position([x as f32, y as f32]);
            }
            if state.maximized {
                viewport = viewport.with_maximized(true);
            }
            tracing::info!("Restored window state from previous session");
        } else {
            // Default to maximized on first run
            viewport = viewport
                .with_inner_size([1100.0, 700.0])
                .with_maximized(true);
        }

        eframe::NativeOptions {
            viewport,
            ..Default::default()
        }
    }

    /// Mark that settings have changed and need to be saved
    pub fn mark_settings_changed(&mut self) {
        self.pending_save = true;
    }

    /// Auto-save settings if there are pending changes (debounced)
    fn auto_save_settings(&mut self) {
        const AUTO_SAVE_DELAY: Duration = Duration::from_secs(2);

        if !self.pending_save {
            return;
        }

        // Only save if enough time has passed since last change
        if self.last_auto_save.elapsed() < AUTO_SAVE_DELAY {
            return;
        }

        if let Some(ref config_dir) = self.config_dir {
            let config_path = config_dir.join("config.toml");
            let state = self.shared_state.read();
            if let Err(e) = crate::config::save_config(&state.config, &config_path) {
                tracing::error!("Failed to auto-save config: {}", e);
            } else {
                tracing::debug!("Auto-saved configuration");
                self.pending_save = false;
                self.last_auto_save = Instant::now();
            }
        }
    }

    /// Auto-save profile zones if they've been modified (debounced)
    fn auto_save_profile_zones(&mut self) {
        const ZONE_SAVE_DELAY: Duration = Duration::from_secs(2);

        let zones_dirty = self.dashboard_state.vision.zones_dirty;

        if !zones_dirty {
            return;
        }

        // Only save if enough time has passed since last change
        if self.last_profile_save.elapsed() < ZONE_SAVE_DELAY {
            return;
        }

        if let (Some(ref mut profile), Some(ref profiles_dir)) =
            (&mut self.active_profile, &self.profiles_dir)
        {
            // Update profile with current zones from vision state
            profile.ocr_regions = self.dashboard_state.vision.ocr_zones.clone();

            let profile_path = profiles_dir.join(format!("{}.json", profile.id));
            if let Err(e) = crate::storage::profiles::save_profile(profile, &profile_path) {
                tracing::error!("Failed to save profile: {}", e);
            } else {
                tracing::info!(
                    "Saved {} zones to profile '{}'",
                    profile.ocr_regions.len(),
                    profile.name
                );
                self.dashboard_state.vision.zones_dirty = false;
                self.last_profile_save = Instant::now();
            }
        }
    }

    /// Save window state periodically (debounced, only when changed)
    fn save_window_state(&mut self, ctx: &egui::Context) {
        const WINDOW_SAVE_INTERVAL: Duration = Duration::from_secs(5);

        // Only check periodically
        if self.last_window_save.elapsed() < WINDOW_SAVE_INTERVAL {
            return;
        }

        self.last_window_save = Instant::now();

        // Get current window state from egui - extract values inside the closure
        let (outer_rect, maximized) = ctx.input(|i| {
            let vp = i.viewport();
            (vp.outer_rect, vp.maximized.unwrap_or(false))
        });

        let current_state = WindowState {
            position: outer_rect.map(|r| (r.left() as i32, r.top() as i32)),
            size: outer_rect.map(|r| (r.width(), r.height())),
            maximized,
        };

        // Only save if state changed
        let should_save = match &self.last_window_state {
            Some(last) => {
                last.position != current_state.position
                    || last.size != current_state.size
                    || last.maximized != current_state.maximized
            }
            None => true,
        };

        if should_save {
            if let Some(ref config_dir) = self.config_dir {
                let state_path = config_dir.join("window_state.toml");
                if let Err(e) = crate::config::save_window_state(&current_state, &state_path) {
                    tracing::error!("Failed to save window state: {}", e);
                } else {
                    tracing::debug!("Saved window state");
                    self.last_window_state = Some(current_state);
                }
            }
        }
    }

    /// Sync dashboard and vision state to config (for auto-save)
    /// Returns true if any changes were detected
    fn sync_dashboard_state_to_config(&mut self) {
        // Build current vision settings from dashboard state
        let current_vision = crate::config::VisionSettings {
            backend: self.dashboard_state.vision.selected_backend,
            granularity: match self.dashboard_state.vision.ocr_granularity {
                crate::dashboard::state::OcrGranularity::Word => crate::vision::OcrGranularity::Word,
                crate::dashboard::state::OcrGranularity::Line => crate::vision::OcrGranularity::Line,
            },
            match_threshold: self.dashboard_state.vision.match_threshold,
            show_bounding_boxes: self.dashboard_state.vision.show_bounding_boxes,
            auto_run_ocr: self.dashboard_state.vision.auto_run_ocr,
            preprocessing: self.dashboard_state.vision.preprocessing.clone(),
        };

        let current_view = self.dashboard_state.current_view;

        // Check for vision settings changes
        let vision_changed = match &self.last_synced_vision {
            Some(last) => {
                last.backend != current_vision.backend
                    || last.granularity != current_vision.granularity
                    || (last.match_threshold - current_vision.match_threshold).abs() > 0.001
                    || last.show_bounding_boxes != current_vision.show_bounding_boxes
                    || last.preprocessing != current_vision.preprocessing
            }
            None => true,
        };

        // Check for view changes
        let view_changed = match &self.last_synced_view {
            Some(last) => *last != current_view,
            None => true,
        };

        if vision_changed || view_changed {
            let mut state = self.shared_state.write();

            // Sync vision settings
            state.config.vision = current_vision.clone();

            // Sync dashboard settings
            state.config.dashboard.last_view = current_view.to_setting();

            // Sync active profile ID
            if let Some(ref profile) = self.active_profile {
                state.config.dashboard.active_profile_id = Some(profile.id.clone());
            }

            drop(state);

            // Update tracking state
            self.last_synced_vision = Some(current_vision);
            self.last_synced_view = Some(current_view);

            // Mark for auto-save
            self.pending_save = true;
            self.last_auto_save = Instant::now();

            tracing::debug!("Dashboard state changed, marked for save");
        }
    }
}

impl eframe::App for DashboardApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Apply theme once
        if !self.theme_applied {
            theme::apply_theme(ctx);
            self.theme_applied = true;
        }

        // Poll for hotkey events
        self.poll_hotkeys();

        // Process commands from UI
        self.process_capture_commands();
        self.process_overlay_commands();
        self.process_profile_commands();
        self.process_test_tip();
        self.process_vision_commands();
        self.process_zone_commands();
        self.process_auto_configure();

        // Sync overlay config changes to running overlay
        self.sync_overlay_config();

        // Update capture statistics if capturing
        self.update_capture_stats();

        // Check if overlay thread has stopped
        self.check_overlay_status();

        // Sync dashboard/vision state to config before saving
        self.sync_dashboard_state_to_config();

        // Auto-save settings if needed
        self.auto_save_settings();

        // Auto-save profile labels if needed
        self.auto_save_profile_zones();

        // Save window state periodically
        self.save_window_state(ctx);

        // Check if settings view has unsaved changes and mark for auto-save
        if self.dashboard_state.settings.has_unsaved_changes {
            self.pending_save = true;
            self.dashboard_state.settings.has_unsaved_changes = false;
        }

        // Request continuous repaint when capturing or when there are pending saves
        if self.is_capturing() || self.pending_save || self.dashboard_state.vision.zones_dirty {
            ctx.request_repaint();
        }

        // Sidebar panel
        egui::SidePanel::left("sidebar")
            .resizable(false)
            .default_width(180.0)
            .show(ctx, |ui| {
                render_sidebar(ui, &mut self.dashboard_state.current_view);
            });

        // Main content panel
        egui::CentralPanel::default().show(ctx, |ui| {
            // Add padding around content
            egui::Frame::none()
                .inner_margin(24.0)
                .show(ui, |ui| {
                    match self.dashboard_state.current_view {
                        DashboardView::Home => {
                            render_home_view(
                                ui,
                                &mut self.dashboard_state.home,
                                &self.shared_state,
                            );
                        }
                        DashboardView::Capture => {
                            render_capture_view(
                                ui,
                                &mut self.dashboard_state.capture,
                                &self.shared_state,
                                &self.capture_manager,
                            );
                        }
                        DashboardView::Overlay => {
                            render_overlay_view(
                                ui,
                                &mut self.dashboard_state.overlay,
                                &self.shared_state,
                            );
                        }
                        DashboardView::Vision => {
                            render_vision_view(
                                ui,
                                &mut self.dashboard_state.vision,
                                &self.shared_state,
                                &self.capture_manager,
                            );
                        }
                        DashboardView::Profiles => {
                            render_profiles_view(
                                ui,
                                &mut self.dashboard_state.profiles,
                                &self.shared_state,
                            );
                        }
                        DashboardView::Settings => {
                            render_settings_view(
                                ui,
                                &mut self.dashboard_state.settings,
                                &self.shared_state,
                            );
                        }
                    }
                });
        });
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        // Save any pending config changes
        if self.pending_save {
            if let Some(ref config_dir) = self.config_dir {
                let config_path = config_dir.join("config.toml");
                let state = self.shared_state.read();
                if let Err(e) = crate::config::save_config(&state.config, &config_path) {
                    tracing::error!("Failed to save config on exit: {}", e);
                } else {
                    tracing::info!("Saved configuration on exit");
                }
            }
        }

        // Save any pending zone changes
        if self.dashboard_state.vision.zones_dirty {
            if let (Some(ref mut profile), Some(ref profiles_dir)) =
                (&mut self.active_profile, &self.profiles_dir)
            {
                profile.ocr_regions = self.dashboard_state.vision.ocr_zones.clone();
                let profile_path = profiles_dir.join(format!("{}.json", profile.id));
                if let Err(e) = crate::storage::profiles::save_profile(profile, &profile_path) {
                    tracing::error!("Failed to save profile zones on exit: {}", e);
                } else {
                    tracing::info!(
                        "Saved {} zones to profile on exit",
                        profile.ocr_regions.len()
                    );
                }
            }
        }

        // Note: Window state is saved via the run_dashboard function
        // since we need access to the context there
        tracing::info!("Dashboard shutting down");
    }
}

impl DashboardApp {
    /// Process capture commands from the UI
    fn process_capture_commands(&mut self) {
        use crate::shared::CaptureCommand;

        let command = {
            let mut state = self.shared_state.write();
            state.runtime.capture_command.take()
        };

        if let Some(cmd) = command {
            match cmd {
                CaptureCommand::Start => {
                    if let Err(e) = self.start_capture() {
                        let mut state = self.shared_state.write();
                        state.runtime.set_error(e);
                    }
                }
                CaptureCommand::Stop => {
                    self.stop_capture();
                }
            }
        }
    }

    /// Process overlay commands from the UI
    fn process_overlay_commands(&mut self) {
        use crate::shared::OverlayCommand;

        let command = {
            let mut state = self.shared_state.write();
            state.runtime.overlay_command.take()
        };

        if let Some(cmd) = command {
            match cmd {
                OverlayCommand::Start => {
                    if let Err(e) = self.start_overlay() {
                        let mut state = self.shared_state.write();
                        state.runtime.set_error(e);
                    }
                }
                OverlayCommand::Stop => {
                    self.stop_overlay();
                }
                OverlayCommand::ToggleVisibility => {
                    let mut state = self.shared_state.write();
                    state.runtime.overlay_visible = !state.runtime.overlay_visible;
                }
            }
        }
    }

    /// Check if overlay thread has stopped unexpectedly
    fn check_overlay_status(&mut self) {
        if self.overlay_manager.is_some() && !self.is_overlay_running() {
            // Overlay thread has stopped
            self.overlay_manager = None;
            self.overlay_handle = None;

            let mut state = self.shared_state.write();
            state.runtime.is_overlay_running = false;
            state.runtime.overlay_visible = false;
        }
    }

    /// Process profile commands from the UI (activate/deactivate)
    fn process_profile_commands(&mut self) {
        use crate::dashboard::state::ProfileAction;

        let action = self.dashboard_state.profiles.pending_action.take();

        if let Some(action) = action {
            match action {
                ProfileAction::Activate(profile_id) => {
                    self.activate_profile(&profile_id);
                }
                ProfileAction::Deactivate => {
                    self.deactivate_profile();
                }
            }
        }
    }

    /// Sync overlay config from shared state to the running overlay (only when changed)
    fn sync_overlay_config(&mut self) {
        if let Some(manager) = &self.overlay_manager {
            let config = self.shared_state.read().overlay_config.clone();

            // Only sync if config has changed
            let should_sync = match &self.last_synced_overlay_config {
                Some(last) => last != &config,
                None => true,
            };

            if should_sync {
                manager.set_config(config.clone());
                self.last_synced_overlay_config = Some(config);
            }
        }
    }

    /// Process test tip request
    fn process_test_tip(&mut self) {
        let should_send = {
            let mut state = self.shared_state.write();
            let send = state.runtime.send_test_tip;
            state.runtime.send_test_tip = false;
            send
        };

        if should_send {
            if let Some(manager) = &self.overlay_manager {
                let tip = Tip {
                    id: format!("test_{}", std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis()),
                    message: "This is a test tip from GamersToolKit!".to_string(),
                    priority: 50,
                    duration_ms: Some(5000),
                    play_sound: false,
                };
                manager.show_tip(tip);

                // Update tips displayed count
                let mut state = self.shared_state.write();
                state.runtime.tips_displayed += 1;
            }
        }
    }

    /// Poll for global hotkey events
    fn poll_hotkeys(&mut self) {
        if let Some(ref hotkey_manager) = self.hotkey_manager {
            use crate::hotkey::HotkeyEvent;

            match hotkey_manager.poll_events() {
                HotkeyEvent::None => {}
                HotkeyEvent::ToggleOverlay => {
                    // Already handled in poll_events
                }
                HotkeyEvent::EnterZoneSelection => {
                    // Request zone selection mode
                    self.dashboard_state.vision.pending_zone_selection_mode = true;
                }
            }
        }
    }

    /// Process vision/OCR commands from the UI
    fn process_vision_commands(&mut self) {
        use crate::vision::OcrBackend;

        let vision_state = &mut self.dashboard_state.vision;

        // Update model status from model manager (for PaddleOCR)
        if let Some(ref manager) = self.model_manager {
            vision_state.detection_model_ready = manager.is_model_available(ModelType::Detection);
            vision_state.recognition_model_ready = manager.is_model_available(ModelType::Recognition);
            vision_state.models_ready = manager.are_models_ready();
        }

        // Update OCR initialized status based on backend
        if let Some(ref pipeline) = self.vision_pipeline {
            vision_state.ocr_initialized = pipeline.is_ocr_ready() && pipeline.backend() == OcrBackend::PaddleOcr;
            vision_state.windows_ocr_initialized = pipeline.is_ocr_ready() && pipeline.backend() == OcrBackend::WindowsOcr;
        } else {
            vision_state.ocr_initialized = false;
            vision_state.windows_ocr_initialized = false;
        }

        // Handle model download request (PaddleOCR only)
        if vision_state.pending_download {
            vision_state.pending_download = false;
            vision_state.is_downloading = true;
            vision_state.last_error = None;

            if let Some(ref manager) = self.model_manager {
                // Download models (blocking for now - could be made async)
                match manager.ensure_all_models() {
                    Ok(()) => {
                        vision_state.is_downloading = false;
                        vision_state.download_progress = 1.0;
                        tracing::info!("OCR models downloaded successfully");
                    }
                    Err(e) => {
                        vision_state.is_downloading = false;
                        vision_state.last_error = Some(format!("Download failed: {}", e));
                        tracing::error!("Failed to download OCR models: {}", e);
                    }
                }
            } else {
                vision_state.is_downloading = false;
                vision_state.last_error = Some("Model manager not initialized".to_string());
            }
        }

        // Handle OCR initialization request - based on selected backend
        if vision_state.pending_init {
            vision_state.pending_init = false;
            vision_state.last_error = None;

            let selected_backend = vision_state.selected_backend;

            // Create or update pipeline with selected backend
            let mut pipeline = match self.vision_pipeline.take() {
                Some(p) => p,
                None => match VisionPipeline::new() {
                    Ok(p) => p,
                    Err(e) => {
                        vision_state.last_error = Some(format!("Pipeline creation failed: {}", e));
                        tracing::error!("Failed to create vision pipeline: {}", e);
                        return;
                    }
                }
            };

            // Set the backend
            pipeline.set_backend(selected_backend);

            // Initialize the selected backend
            if let Err(e) = pipeline.init_ocr() {
                vision_state.last_error = Some(format!("OCR init failed: {}", e));
                tracing::error!("Failed to initialize {:?} OCR: {}", selected_backend, e);
            } else {
                match selected_backend {
                    OcrBackend::WindowsOcr => {
                        vision_state.windows_ocr_initialized = true;
                        tracing::info!("Windows OCR engine initialized successfully");
                    }
                    OcrBackend::PaddleOcr => {
                        vision_state.ocr_initialized = true;
                        tracing::info!("PaddleOCR engine initialized successfully");
                    }
                }
            }

            self.vision_pipeline = Some(pipeline);
        }
    }

    /// Process zone selection and OCR commands
    fn process_zone_commands(&mut self) {
        // Handle request to enter zone selection mode
        if self.dashboard_state.vision.pending_zone_selection_mode {
            self.dashboard_state.vision.pending_zone_selection_mode = false;

            // Auto-start overlay if not running
            if self.overlay_manager.is_none() {
                tracing::info!("Auto-starting overlay for zone selection");
                if let Err(e) = self.start_overlay() {
                    tracing::error!("Failed to start overlay for zone selection: {}", e);
                    self.dashboard_state.vision.zone_selection_error =
                        Some(format!("Failed to start overlay: {}", e));
                    return;
                }
                // Give the overlay a moment to initialize
                std::thread::sleep(std::time::Duration::from_millis(100));
            }

            if let Some(ref manager) = self.overlay_manager {
                // Send existing zones to overlay for display
                let existing_zones: Vec<(String, (f32, f32, f32, f32))> = self.dashboard_state.vision
                    .ocr_zones
                    .iter()
                    .map(|z| (z.name.clone(), z.bounds))
                    .collect();

                // Get capture frame dimensions for proper coordinate normalization
                let capture_size = {
                    let w = self.dashboard_state.vision.last_frame_width;
                    let h = self.dashboard_state.vision.last_frame_height;
                    if w > 0 && h > 0 {
                        Some((w, h))
                    } else {
                        None
                    }
                };

                manager.enter_zone_selection_mode(existing_zones, capture_size);
                self.dashboard_state.vision.zone_selection.is_selecting = true;
                self.dashboard_state.vision.zone_selection_error = None;
                tracing::info!("Requested zone selection mode with capture_size: {:?}", capture_size);
            } else {
                tracing::warn!("Cannot enter zone selection mode: overlay not running");
                self.dashboard_state.vision.zone_selection_error =
                    Some("Overlay failed to start".to_string());
            }
        }

        let vision_state = &mut self.dashboard_state.vision;

        // Poll for zone selection results from overlay
        if let Some(ref manager) = self.overlay_manager {
            if let Some(result) = manager.poll_zone_selection_result() {
                match result {
                    ZoneSelectionResult::Completed { bounds } => {
                        // Check if we're repositioning an existing zone
                        if let Some(idx) = vision_state.zone_selection.repositioning_zone_index {
                            // Update existing zone's bounds
                            if let Some(zone) = vision_state.ocr_zones.get_mut(idx) {
                                zone.bounds = bounds;
                                vision_state.zones_dirty = true;
                                tracing::info!(
                                    "Zone '{}' repositioned: ({:.2}, {:.2}, {:.2}, {:.2})",
                                    zone.name,
                                    bounds.0,
                                    bounds.1,
                                    bounds.2,
                                    bounds.3
                                );
                            }
                            vision_state.zone_selection.repositioning_zone_index = None;
                        } else {
                            // Creating a new zone - show naming dialog
                            vision_state.zone_selection.current_selection = Some(bounds);
                            vision_state.zone_selection.show_naming_dialog = true;
                            tracing::info!(
                                "Zone selection completed: ({:.2}, {:.2}, {:.2}, {:.2})",
                                bounds.0,
                                bounds.1,
                                bounds.2,
                                bounds.3
                            );
                        }
                        vision_state.zone_selection.is_selecting = false;
                    }
                    ZoneSelectionResult::Cancelled => {
                        vision_state.zone_selection.is_selecting = false;
                        vision_state.zone_selection.current_selection = None;
                        vision_state.zone_selection.repositioning_zone_index = None;
                        tracing::info!("Zone selection cancelled");
                    }
                }
            }
        }

        // Run zone OCR for enabled zones when we have frame data
        // This runs on every frame when auto_run_ocr is enabled
        self.process_zone_ocr();
    }

    /// Process OCR for all enabled zones
    fn process_zone_ocr(&mut self) {
        use crate::vision::OcrBackend;

        let vision_state = &mut self.dashboard_state.vision;

        // Only process if zones are defined
        if vision_state.ocr_zones.is_empty() {
            return;
        }

        // Check if any zones are enabled
        let has_enabled_zones = vision_state.ocr_zones.iter().any(|z| z.enabled);
        if !has_enabled_zones {
            return;
        }

        // Don't process while another OCR is running
        if vision_state.is_processing {
            return;
        }

        let selected_backend = vision_state.selected_backend;
        let backend_ready = match selected_backend {
            OcrBackend::WindowsOcr => vision_state.windows_ocr_initialized,
            OcrBackend::PaddleOcr => vision_state.ocr_initialized,
        };

        // Auto-initialize OCR if not ready but zones are enabled
        if !backend_ready {
            // Create pipeline if needed
            if self.vision_pipeline.is_none() {
                match VisionPipeline::new() {
                    Ok(p) => {
                        self.vision_pipeline = Some(p);
                        tracing::info!("Created vision pipeline for zone OCR");
                    }
                    Err(e) => {
                        tracing::debug!("Failed to create vision pipeline: {}", e);
                        return;
                    }
                }
            }

            // Initialize OCR backend
            if let Some(ref mut pipeline) = self.vision_pipeline {
                pipeline.set_backend(selected_backend);
                if let Err(e) = pipeline.init_ocr() {
                    tracing::debug!("Failed to initialize OCR for zone processing: {}", e);
                    return;
                }

                match selected_backend {
                    OcrBackend::WindowsOcr => {
                        vision_state.windows_ocr_initialized = true;
                        tracing::info!("Auto-initialized Windows OCR for zone processing");
                    }
                    OcrBackend::PaddleOcr => {
                        vision_state.ocr_initialized = true;
                        tracing::info!("Auto-initialized PaddleOCR for zone processing");
                    }
                }
            }
        }

        // Get a fresh frame from capture manager (zone OCR runs independently of Vision view)
        let frame = {
            let capture_guard = self.capture_manager.lock();
            if let Some(ref capture) = *capture_guard {
                capture.try_next_frame()
            } else {
                None
            }
        };

        let Some(frame) = frame else {
            return;
        };

        let Some(ref mut pipeline) = self.vision_pipeline else {
            return;
        };

        // Ensure backend is synced with user selection
        pipeline.set_backend(selected_backend);

        let frame_width = frame.width;
        let frame_height = frame.height;

        if frame_width == 0 || frame_height == 0 {
            return;
        }

        // Process each enabled zone
        for zone in &vision_state.ocr_zones {
            if !zone.enabled {
                continue;
            }

            // Convert normalized bounds to pixel coordinates
            let x = (zone.bounds.0 * frame_width as f32) as u32;
            let y = (zone.bounds.1 * frame_height as f32) as u32;
            let w = (zone.bounds.2 * frame_width as f32) as u32;
            let h = (zone.bounds.3 * frame_height as f32) as u32;

            // Ensure minimum size
            if w < 5 || h < 5 {
                continue;
            }

            // Get preprocessing settings: use zone's custom settings if available, otherwise global
            let preprocessing = zone.preprocessing.as_ref()
                .or(Some(&vision_state.preprocessing))
                .filter(|pp| pp.enabled);

            tracing::info!(
                "Zone '{}': processing region ({}, {}) {}x{} (frame: {}x{})",
                zone.name, x, y, w, h, frame_width, frame_height
            );

            // Run OCR on the zone region with preprocessing
            match pipeline.process_region_with_preprocessing(&frame, x, y, w, h, preprocessing) {
                Ok(result) => {
                    tracing::info!(
                        "Zone '{}': OCR returned {} text regions",
                        zone.name,
                        result.text_regions.len()
                    );
                    for (i, r) in result.text_regions.iter().enumerate() {
                        tracing::info!("  Region {}: '{}' (conf: {:.2})", i, r.text, r.confidence);
                    }

                    // Combine all detected text
                    let raw_text: String = result
                        .text_regions
                        .iter()
                        .map(|r| r.text.as_str())
                        .collect::<Vec<_>>()
                        .join(" ");

                    // Filter text based on content type
                    let text = filter_text_by_content_type(&raw_text, &zone.content_type);

                    // Update zone result
                    vision_state.zone_ocr_results.insert(
                        zone.id.clone(),
                        ZoneOcrResult {
                            zone_id: zone.id.clone(),
                            zone_name: zone.name.clone(),
                            text,
                            last_updated: Instant::now(),
                        },
                    );
                }
                Err(e) => {
                    tracing::info!("Zone OCR failed for '{}': {}", zone.name, e);
                }
            }
        }
    }

    /// Process auto-configure for a zone
    /// This iterates through different OCR settings until text is detected
    fn process_auto_configure(&mut self) {
        use crate::config::OcrPreprocessing;
        use crate::vision::OcrBackend;

        // Check if auto-configure is active
        let auto_config = match self.dashboard_state.vision.zone_selection.auto_configure.as_mut() {
            Some(ac) if ac.current_step != AutoConfigureStep::Completed => ac,
            _ => return,
        };

        let zone_idx = auto_config.zone_index;

        // Validate zone exists
        let zone = match self.dashboard_state.vision.ocr_zones.get(zone_idx) {
            Some(z) => z.clone(),
            None => {
                if let Some(ref mut ac) = self.dashboard_state.vision.zone_selection.auto_configure {
                    ac.current_step = AutoConfigureStep::Completed;
                    ac.error_message = Some("Zone not found".to_string());
                }
                return;
            }
        };

        // Ensure OCR is initialized
        let vision_state = &self.dashboard_state.vision;
        let selected_backend = vision_state.selected_backend;
        let backend_ready = match selected_backend {
            OcrBackend::WindowsOcr => vision_state.windows_ocr_initialized,
            OcrBackend::PaddleOcr => vision_state.ocr_initialized,
        };

        if !backend_ready {
            // Try to initialize
            if self.vision_pipeline.is_none() {
                match VisionPipeline::new() {
                    Ok(p) => self.vision_pipeline = Some(p),
                    Err(e) => {
                        if let Some(ref mut ac) = self.dashboard_state.vision.zone_selection.auto_configure {
                            ac.current_step = AutoConfigureStep::Completed;
                            ac.error_message = Some(format!("Failed to create pipeline: {}", e));
                        }
                        return;
                    }
                }
            }

            if let Some(ref mut pipeline) = self.vision_pipeline {
                pipeline.set_backend(selected_backend);
                if let Err(e) = pipeline.init_ocr() {
                    if let Some(ref mut ac) = self.dashboard_state.vision.zone_selection.auto_configure {
                        ac.current_step = AutoConfigureStep::Completed;
                        ac.error_message = Some(format!("Failed to init OCR: {}", e));
                    }
                    return;
                }

                match selected_backend {
                    OcrBackend::WindowsOcr => {
                        self.dashboard_state.vision.windows_ocr_initialized = true;
                    }
                    OcrBackend::PaddleOcr => {
                        self.dashboard_state.vision.ocr_initialized = true;
                    }
                }
            }
        }

        // Use stored frame data from vision state (more reliable than try_next_frame)
        let frame = {
            let vision = &self.dashboard_state.vision;
            if let Some(ref data) = vision.last_frame_data {
                if vision.last_frame_width > 0 && vision.last_frame_height > 0 {
                    Some(crate::capture::CapturedFrame::new(
                        data.clone(),
                        vision.last_frame_width,
                        vision.last_frame_height,
                    ))
                } else {
                    None
                }
            } else {
                None
            }
        };

        let Some(frame) = frame else {
            if let Some(ref mut ac) = self.dashboard_state.vision.zone_selection.auto_configure {
                ac.status_message = "Waiting for capture frame... (open Vision tab)".to_string();
            }
            return;
        };

        let Some(ref mut pipeline) = self.vision_pipeline else {
            return;
        };

        pipeline.set_backend(selected_backend);

        let frame_width = frame.width;
        let frame_height = frame.height;

        if frame_width == 0 || frame_height == 0 {
            return;
        }

        // Convert zone bounds to pixels
        let x = (zone.bounds.0 * frame_width as f32) as u32;
        let y = (zone.bounds.1 * frame_height as f32) as u32;
        let w = (zone.bounds.2 * frame_width as f32) as u32;
        let h = (zone.bounds.3 * frame_height as f32) as u32;

        if w < 5 || h < 5 {
            if let Some(ref mut ac) = self.dashboard_state.vision.zone_selection.auto_configure {
                ac.current_step = AutoConfigureStep::Completed;
                ac.error_message = Some("Zone too small".to_string());
            }
            return;
        }

        // Get current settings to test from auto_configure state
        let ac = self.dashboard_state.vision.zone_selection.auto_configure.as_ref().unwrap();
        let test_preprocessing = if ac.current_preprocessing_enabled {
            Some(OcrPreprocessing {
                enabled: true,
                grayscale: ac.current_grayscale,
                contrast: ac.current_contrast,
                sharpen: 0.0,
                invert: ac.current_invert,
                scale: ac.current_scale,
            })
        } else {
            Some(OcrPreprocessing {
                enabled: false,
                grayscale: false,
                contrast: 1.0,
                sharpen: 0.0,
                invert: false,
                scale: ac.current_scale,
            })
        };

        // Update status message
        let status = format!(
            "Testing: scale={}x, pp={}, gray={}, inv={}, contrast={:.1}",
            ac.current_scale,
            if ac.current_preprocessing_enabled { "on" } else { "off" },
            if ac.current_grayscale { "Y" } else { "N" },
            if ac.current_invert { "Y" } else { "N" },
            ac.current_contrast
        );

        // Run OCR with current settings
        let result = pipeline.process_region_with_preprocessing(
            &frame,
            x, y, w, h,
            test_preprocessing.as_ref(),
        );

        match result {
            Ok(ocr_result) => {
                // Check if we got text
                let combined_text: String = ocr_result
                    .text_regions
                    .iter()
                    .map(|r| r.text.as_str())
                    .collect::<Vec<_>>()
                    .join(" ");

                // Calculate average confidence
                let avg_confidence = if ocr_result.text_regions.is_empty() {
                    0.0
                } else {
                    ocr_result.text_regions.iter().map(|r| r.confidence).sum::<f32>()
                        / ocr_result.text_regions.len() as f32
                };

                let filtered_text = filter_text_by_content_type(&combined_text, &zone.content_type);

                if !filtered_text.trim().is_empty() {
                    // Found text - check if this is better than our best so far
                    let ac = self.dashboard_state.vision.zone_selection.auto_configure.as_mut().unwrap();

                    if avg_confidence > ac.best_confidence {
                        tracing::info!(
                            "Auto-configure found better config for '{}': text='{}', confidence={:.2}, settings: {:?}",
                            zone.name,
                            filtered_text,
                            avg_confidence,
                            test_preprocessing
                        );
                        ac.best_confidence = avg_confidence;
                        ac.best_text = filtered_text;
                        ac.best_preprocessing = test_preprocessing;
                    }
                }
            }
            Err(e) => {
                tracing::debug!("Auto-configure OCR failed: {}", e);
            }
        }

        // Advance to next configuration
        let ac = self.dashboard_state.vision.zone_selection.auto_configure.as_mut().unwrap();
        ac.status_message = status;
        ac.current_step = AutoConfigureStep::Testing;
        ac.current_combination += 1;

        // Advance settings: scale -> preprocessing -> grayscale -> invert -> contrast
        // Contrast values: 1.0, 1.5, 2.0
        let contrast_values = [1.0, 1.5, 2.0];

        // Find current contrast index
        let current_contrast_idx = contrast_values.iter()
            .position(|&c| (c - ac.current_contrast).abs() < 0.01)
            .unwrap_or(0);

        // Try next contrast
        if current_contrast_idx + 1 < contrast_values.len() {
            ac.current_contrast = contrast_values[current_contrast_idx + 1];
            return;
        }

        // Reset contrast, try next invert
        ac.current_contrast = 1.0;
        if !ac.current_invert {
            ac.current_invert = true;
            return;
        }

        // Reset invert, try next grayscale
        ac.current_invert = false;
        if !ac.current_grayscale {
            ac.current_grayscale = true;
            return;
        }

        // Reset grayscale, try next preprocessing state
        ac.current_grayscale = false;
        if !ac.current_preprocessing_enabled {
            ac.current_preprocessing_enabled = true;
            return;
        }

        // Reset preprocessing, try next scale
        ac.current_preprocessing_enabled = false;
        if ac.current_scale < 4 {
            ac.current_scale += 1;
            return;
        }

        // All combinations exhausted - apply best configuration if found
        if ac.best_preprocessing.is_some() {
            let best_pp = ac.best_preprocessing.clone();
            let best_text = ac.best_text.clone();
            let best_conf = ac.best_confidence;

            // Apply the best settings to the zone
            if let Some(zone) = self.dashboard_state.vision.ocr_zones.get_mut(zone_idx) {
                zone.preprocessing = best_pp;
                self.dashboard_state.vision.zones_dirty = true;
            }

            ac.current_step = AutoConfigureStep::Completed;
            ac.success = true;
            ac.status_message = format!("Best: '{}' ({:.0}%)", best_text, best_conf * 100.0);

            tracing::info!(
                "Auto-configure completed for zone {}: applied best config with confidence {:.2}",
                zone_idx,
                best_conf
            );
        } else {
            ac.current_step = AutoConfigureStep::Completed;
            ac.success = false;
            ac.error_message = Some("No settings found that produce text".to_string());
        }
    }
}

/// Run the dashboard application
pub fn run_dashboard(shared_state: Arc<RwLock<SharedAppState>>) -> Result<(), eframe::Error> {
    let app = DashboardApp::new(shared_state);
    eframe::run_native(
        "GamersToolKit Dashboard",
        DashboardApp::options(),
        Box::new(|_cc| Ok(Box::new(app))),
    )
}

/// Filter OCR text based on the expected content type
/// This helps clean up OCR results by removing characters that don't match the expected type
fn filter_text_by_content_type(text: &str, content_type: &ContentType) -> String {
    match content_type {
        ContentType::Text => {
            // For text, just trim whitespace
            text.trim().to_string()
        }
        ContentType::Number => {
            // Keep only digits, decimal points, commas (for thousands), and minus sign
            // Also handle common OCR mistakes: O->0, l/I->1, S->5, B->8
            let cleaned: String = text
                .chars()
                .map(|c| match c {
                    'O' | 'o' => '0',
                    'l' | 'I' | '|' => '1',
                    'S' | 's' => '5',
                    'B' => '8',
                    _ => c,
                })
                .filter(|c| c.is_ascii_digit() || *c == '.' || *c == ',' || *c == '-')
                .collect();
            cleaned
        }
        ContentType::Percentage => {
            // Keep digits, decimal points, and percent sign
            let cleaned: String = text
                .chars()
                .map(|c| match c {
                    'O' | 'o' => '0',
                    'l' | 'I' | '|' => '1',
                    'S' | 's' => '5',
                    'B' => '8',
                    _ => c,
                })
                .filter(|c| c.is_ascii_digit() || *c == '.' || *c == '%' || *c == '-')
                .collect();
            // Ensure % is at the end if present anywhere
            if cleaned.contains('%') {
                let without_percent: String = cleaned.chars().filter(|c| *c != '%').collect();
                format!("{}%", without_percent)
            } else {
                cleaned
            }
        }
        ContentType::Time => {
            // Keep digits and colons for time formats like 12:34 or 1:23:45
            let cleaned: String = text
                .chars()
                .map(|c| match c {
                    'O' | 'o' => '0',
                    'l' | 'I' | '|' => '1',
                    _ => c,
                })
                .filter(|c| c.is_ascii_digit() || *c == ':')
                .collect();
            cleaned
        }
    }
}
