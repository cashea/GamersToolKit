//! Application Coordinator
//!
//! Manages the lifecycle of both the dashboard and overlay windows,
//! including shared state and inter-component communication.

use anyhow::Result;
use crossbeam_channel::{unbounded, Receiver, Sender};
use parking_lot::RwLock;
use std::sync::Arc;
use std::thread::JoinHandle;
use tracing::info;

use crate::analysis::Tip;
use crate::config::AppConfig;
use crate::overlay::{OverlayConfig, OverlayManager};
use crate::shared::{DashboardToOverlay, OverlayToDashboard, SharedAppState};

/// Main application coordinator
pub struct GamersToolKitApp {
    /// Shared state between dashboard and overlay
    pub shared_state: Arc<RwLock<SharedAppState>>,
    /// Channel to send messages to overlay
    pub to_overlay: Sender<DashboardToOverlay>,
    /// Channel to receive messages from overlay
    pub from_overlay: Receiver<OverlayToDashboard>,
    /// Handle to overlay thread
    overlay_handle: Option<JoinHandle<()>>,
    /// Overlay manager reference for sending tips
    overlay_manager: Option<Arc<OverlayManager>>,
}

impl GamersToolKitApp {
    /// Create a new application coordinator
    pub fn new(config: AppConfig) -> Result<Self> {
        let shared_state = Arc::new(RwLock::new(SharedAppState::new(config)));
        let (to_overlay, overlay_rx) = unbounded();
        let (overlay_tx, from_overlay) = unbounded();

        Ok(Self {
            shared_state,
            to_overlay,
            from_overlay,
            overlay_handle: None,
            overlay_manager: None,
        })
    }

    /// Start the overlay in a background thread
    pub fn start_overlay(&mut self) -> Result<()> {
        let shared_state = self.shared_state.clone();
        let overlay_config = shared_state.read().overlay_config.clone();

        let manager = Arc::new(OverlayManager::new(overlay_config)?);
        self.overlay_manager = Some(manager.clone());

        // Update runtime state
        {
            let mut state = self.shared_state.write();
            state.runtime.is_overlay_running = true;
            state.runtime.overlay_visible = true;
        }

        let handle = std::thread::spawn(move || {
            info!("Overlay thread starting...");
            if let Err(e) = manager.run() {
                tracing::error!("Overlay error: {}", e);
            }
            info!("Overlay thread exiting...");
        });

        self.overlay_handle = Some(handle);
        info!("Overlay started in background thread");

        Ok(())
    }

    /// Send a tip to the overlay
    pub fn show_tip(&self, tip: Tip) {
        if let Some(manager) = &self.overlay_manager {
            manager.show_tip(tip);
        }
    }

    /// Get a tip sender for external use
    pub fn tip_sender(&self) -> Option<Sender<Tip>> {
        self.overlay_manager.as_ref().map(|m| m.tip_sender())
    }

    /// Update overlay configuration
    pub fn update_overlay_config(&self, config: OverlayConfig) {
        if let Some(manager) = &self.overlay_manager {
            manager.set_config(config.clone());
        }
        self.shared_state.write().overlay_config = config;
    }

    /// Get current shared state
    pub fn state(&self) -> Arc<RwLock<SharedAppState>> {
        self.shared_state.clone()
    }

    /// Check if overlay is running
    pub fn is_overlay_running(&self) -> bool {
        self.overlay_handle
            .as_ref()
            .map(|h| !h.is_finished())
            .unwrap_or(false)
    }
}

impl Drop for GamersToolKitApp {
    fn drop(&mut self) {
        // Signal overlay to stop
        let _ = self.to_overlay.send(DashboardToOverlay::Shutdown);

        // Wait for overlay thread to finish
        if let Some(handle) = self.overlay_handle.take() {
            let _ = handle.join();
        }
    }
}
