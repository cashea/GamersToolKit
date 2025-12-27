//! Shared state and messaging between dashboard and overlay
//!
//! This module provides thread-safe shared state and message passing
//! for communication between the dashboard UI and overlay components.

pub mod state;
pub mod messages;

pub use state::{SharedAppState, RuntimeState, CaptureCommand, OverlayCommand};
pub use messages::{DashboardToOverlay, OverlayToDashboard, OverlayStatus};
