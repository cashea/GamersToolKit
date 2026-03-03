//! Shared state and messaging between dashboard and overlay
//!
//! This module provides thread-safe shared state and message passing
//! for communication between the dashboard UI and overlay components.

pub mod messages;
pub mod state;

pub use messages::{DashboardToOverlay, OverlayToDashboard};
pub use state::{CaptureCommand, OverlayCommand, SharedAppState};
