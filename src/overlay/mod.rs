//! Overlay Presentation Layer
//!
//! Displays tips and alerts using egui_overlay with click passthrough.
//! The overlay is a separate window that doesn't interact with the game.

pub mod widgets;

use anyhow::Result;
use crate::analysis::Tip;

/// Overlay configuration
#[derive(Debug, Clone)]
pub struct OverlayConfig {
    /// Overlay opacity (0.0 - 1.0)
    pub opacity: f32,
    /// Whether overlay is enabled
    pub enabled: bool,
    /// Position offset from corner
    pub offset: (i32, i32),
    /// Which corner to anchor to
    pub anchor: OverlayAnchor,
}

impl Default for OverlayConfig {
    fn default() -> Self {
        Self {
            opacity: 0.9,
            enabled: true,
            offset: (10, 10),
            anchor: OverlayAnchor::TopRight,
        }
    }
}

/// Corner anchor for overlay positioning
#[derive(Debug, Clone, Copy)]
pub enum OverlayAnchor {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

/// Overlay window manager
pub struct OverlayManager {
    config: OverlayConfig,
    active_tips: Vec<Tip>,
}

impl OverlayManager {
    /// Create a new overlay manager
    pub fn new(config: OverlayConfig) -> Result<Self> {
        Ok(Self {
            config,
            active_tips: vec![],
        })
    }

    /// Show a tip on the overlay
    pub fn show_tip(&mut self, tip: Tip) {
        self.active_tips.push(tip);
    }

    /// Clear all tips
    pub fn clear_tips(&mut self) {
        self.active_tips.clear();
    }

    /// Run the overlay event loop (blocking)
    pub fn run(&mut self) -> Result<()> {
        // TODO: Initialize egui_overlay and run event loop
        Ok(())
    }
}
