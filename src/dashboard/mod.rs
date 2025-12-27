//! Dashboard UI Module
//!
//! A full-featured dashboard application for managing GamersToolKit settings,
//! game profiles, capture configuration, and overlay customization.

pub mod app;
pub mod state;
pub mod theme;
pub mod views;
pub mod components;

pub use app::DashboardApp;
pub use state::{DashboardView, DashboardState};
