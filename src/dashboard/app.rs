//! Dashboard application entry point

use eframe::egui;
use parking_lot::{Mutex, RwLock};
use std::sync::Arc;
use std::time::Instant;

use crate::analysis::Tip;
use crate::capture::{CaptureTarget, ScreenCapture};
use crate::dashboard::components::render_sidebar;
use crate::dashboard::state::{DashboardState, DashboardView};
use crate::dashboard::theme;
use crate::dashboard::views::{
    render_capture_view, render_home_view, render_overlay_view,
    render_profiles_view, render_settings_view,
};
use crate::overlay::OverlayManager;
use crate::shared::SharedAppState;
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
        Self {
            shared_state,
            dashboard_state: DashboardState::default(),
            theme_applied: false,
            capture_manager: Arc::new(Mutex::new(None)),
            frame_counter: FrameCounter::default(),
            overlay_manager: None,
            overlay_handle: None,
            last_synced_overlay_config: None,
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
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([1100.0, 700.0])
                .with_min_inner_size([800.0, 500.0])
                .with_title("GamersToolKit Dashboard"),
            ..Default::default()
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

        // Process commands from UI
        self.process_capture_commands();
        self.process_overlay_commands();
        self.process_test_tip();

        // Sync overlay config changes to running overlay
        self.sync_overlay_config();

        // Update capture statistics if capturing
        self.update_capture_stats();

        // Check if overlay thread has stopped
        self.check_overlay_status();

        // Request continuous repaint when capturing to update FPS display
        if self.is_capturing() {
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
                            );
                        }
                        DashboardView::Overlay => {
                            render_overlay_view(
                                ui,
                                &mut self.dashboard_state.overlay,
                                &self.shared_state,
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
