//! Dashboard application entry point

use eframe::egui;
use parking_lot::{Mutex, RwLock};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::analysis::Tip;
use crate::capture::{CaptureTarget, ScreenCapture};
use crate::config::WindowState;
use crate::dashboard::components::render_sidebar;
use crate::dashboard::state::{DashboardState, DashboardView};
use crate::dashboard::theme;
use crate::dashboard::views::{
    render_capture_view, render_home_view, render_overlay_view,
    render_profiles_view, render_settings_view, render_vision_view,
};
use crate::hotkey::HotkeyManager;
use crate::overlay::OverlayManager;
use crate::shared::SharedAppState;
use crate::storage::profiles::GameProfile;
use crate::vision::{VisionPipeline, ModelManager, ModelType, OcrGranularity};
use crate::dashboard::state::OcrResultDisplay;
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

        // Load or create default profile
        let (active_profile, initial_labels) = Self::load_or_create_default_profile(&profiles_dir);

        let mut dashboard_state = DashboardState::default();
        // Load labels from profile into vision state
        dashboard_state.vision.labeled_regions = initial_labels;

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
        }
    }

    /// Load or create the default profile
    fn load_or_create_default_profile(
        profiles_dir: &Option<PathBuf>,
    ) -> (Option<GameProfile>, Vec<crate::storage::profiles::LabeledRegion>) {
        if let Some(ref dir) = profiles_dir {
            let default_path = dir.join("default.json");
            if default_path.exists() {
                match crate::storage::profiles::load_profile(&default_path) {
                    Ok(profile) => {
                        let labels = profile.labeled_regions.clone();
                        tracing::info!("Loaded default profile with {} labels", labels.len());
                        return (Some(profile), labels);
                    }
                    Err(e) => {
                        tracing::warn!("Failed to load default profile: {}", e);
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

            // Save the new default profile
            if let Err(e) = crate::storage::profiles::save_profile(&profile, &default_path) {
                tracing::warn!("Failed to save default profile: {}", e);
            } else {
                tracing::info!("Created new default profile");
            }

            (Some(profile), vec![])
        } else {
            (None, vec![])
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
            viewport = viewport.with_inner_size([1100.0, 700.0]);
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

    /// Auto-save profile labels if they've been modified (debounced)
    /// Also handles immediate save when pending_profile_save is set
    fn auto_save_profile_labels(&mut self) {
        const LABEL_SAVE_DELAY: Duration = Duration::from_secs(2);

        // Check for immediate save request
        let immediate_save = self.dashboard_state.vision.pending_profile_save;
        if immediate_save {
            self.dashboard_state.vision.pending_profile_save = false;
        }

        if !self.dashboard_state.vision.labels_dirty && !immediate_save {
            return;
        }

        // Only save if enough time has passed since last change (unless immediate save requested)
        if !immediate_save && self.last_profile_save.elapsed() < LABEL_SAVE_DELAY {
            return;
        }

        if let (Some(ref mut profile), Some(ref profiles_dir)) =
            (&mut self.active_profile, &self.profiles_dir)
        {
            // Update profile with current labels from vision state
            profile.labeled_regions = self.dashboard_state.vision.labeled_regions.clone();

            let profile_path = profiles_dir.join(format!("{}.json", profile.id));
            if let Err(e) = crate::storage::profiles::save_profile(profile, &profile_path) {
                tracing::error!("Failed to save profile labels: {}", e);
            } else {
                tracing::info!(
                    "Saved {} labels to profile '{}'",
                    profile.labeled_regions.len(),
                    profile.name
                );
                self.dashboard_state.vision.labels_dirty = false;
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
        self.process_test_tip();
        self.process_vision_commands();

        // Sync overlay config changes to running overlay
        self.sync_overlay_config();

        // Update capture statistics if capturing
        self.update_capture_stats();

        // Check if overlay thread has stopped
        self.check_overlay_status();

        // Auto-save settings if needed
        self.auto_save_settings();

        // Auto-save profile labels if needed
        self.auto_save_profile_labels();

        // Save window state periodically
        self.save_window_state(ctx);

        // Check if settings view has unsaved changes and mark for auto-save
        if self.dashboard_state.settings.has_unsaved_changes {
            self.pending_save = true;
            self.dashboard_state.settings.has_unsaved_changes = false;
        }

        // Request continuous repaint when capturing or when there are pending saves
        if self.is_capturing() || self.pending_save || self.dashboard_state.vision.labels_dirty {
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

        // Save any pending profile label changes
        if self.dashboard_state.vision.labels_dirty {
            if let (Some(ref mut profile), Some(ref profiles_dir)) =
                (&mut self.active_profile, &self.profiles_dir)
            {
                profile.labeled_regions = self.dashboard_state.vision.labeled_regions.clone();
                let profile_path = profiles_dir.join(format!("{}.json", profile.id));
                if let Err(e) = crate::storage::profiles::save_profile(profile, &profile_path) {
                    tracing::error!("Failed to save profile labels on exit: {}", e);
                } else {
                    tracing::info!(
                        "Saved {} labels to profile on exit",
                        profile.labeled_regions.len()
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
            hotkey_manager.poll_events();
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

        // Handle OCR run request (manual or auto)
        let backend_ready = match vision_state.selected_backend {
            OcrBackend::WindowsOcr => vision_state.windows_ocr_initialized,
            OcrBackend::PaddleOcr => vision_state.ocr_initialized,
        };
        let should_run_ocr = vision_state.pending_ocr_run
            || (vision_state.auto_run_ocr && vision_state.last_frame_data.is_some());

        if should_run_ocr && backend_ready && self.vision_pipeline.is_some() && !vision_state.is_processing {
            vision_state.pending_ocr_run = false;

            if let Some(ref frame_data) = vision_state.last_frame_data.clone() {
                let width = vision_state.last_frame_width;
                let height = vision_state.last_frame_height;

                if width > 0 && height > 0 {
                    vision_state.is_processing = true;
                    let start = Instant::now();

                    if let Some(ref mut pipeline) = self.vision_pipeline {
                        // Ensure pipeline is using the selected backend
                        pipeline.set_backend(vision_state.selected_backend);

                        // Create a CapturedFrame for processing
                        let frame = crate::capture::frame::CapturedFrame::new(
                            frame_data.clone(),
                            width,
                            height,
                        );

                        // Convert dashboard granularity to vision granularity
                        let granularity = match vision_state.ocr_granularity {
                            crate::dashboard::state::OcrGranularity::Word => OcrGranularity::Word,
                            crate::dashboard::state::OcrGranularity::Line => OcrGranularity::Line,
                        };

                        match pipeline.process_with_granularity(&frame, granularity) {
                            Ok(result) => {
                                // Convert results to display format
                                vision_state.last_ocr_results = result.text_regions
                                    .into_iter()
                                    .map(|r| OcrResultDisplay {
                                        text: r.text,
                                        bounds: r.bounds,
                                        confidence: r.confidence,
                                    })
                                    .collect();
                                vision_state.last_processing_time_ms = start.elapsed().as_millis() as u64;
                                vision_state.last_error = None;

                                // Update labeled regions with current OCR results
                                vision_state.update_labels_from_ocr();
                            }
                            Err(e) => {
                                vision_state.last_error = Some(format!("OCR failed: {}", e));
                                tracing::error!("OCR processing failed: {}", e);
                            }
                        }
                    }

                    vision_state.is_processing = false;
                }
            }
        }

        // Clear frame data after auto-run to prevent repeated processing
        if vision_state.auto_run_ocr && !vision_state.is_processing {
            vision_state.last_frame_data = None;
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
