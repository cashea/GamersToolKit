//! Application Configuration
//!
//! User settings and preferences stored in TOML format.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::overlay::OverlayConfig;
use crate::capture::CaptureConfig;

/// Application settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// General settings
    pub general: GeneralConfig,
    /// Capture settings
    pub capture: CaptureSettings,
    /// Overlay settings
    pub overlay: OverlaySettings,
    /// Performance settings
    pub performance: PerformanceConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            general: GeneralConfig::default(),
            capture: CaptureSettings::default(),
            overlay: OverlaySettings::default(),
            performance: PerformanceConfig::default(),
        }
    }
}

/// General application settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    /// Start minimized to tray
    pub start_minimized: bool,
    /// Auto-start with Windows
    pub auto_start: bool,
    /// Check for updates on startup
    pub check_updates: bool,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            start_minimized: false,
            auto_start: false,
            check_updates: true,
        }
    }
}

/// Capture-related settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureSettings {
    /// Maximum capture FPS
    pub max_fps: u32,
    /// Capture cursor in frames
    pub capture_cursor: bool,
}

impl Default for CaptureSettings {
    fn default() -> Self {
        Self {
            max_fps: 30,
            capture_cursor: false,
        }
    }
}

/// Overlay-related settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverlaySettings {
    /// Overlay enabled
    pub enabled: bool,
    /// Overlay opacity
    pub opacity: f32,
    /// Play sound notifications
    pub sound_enabled: bool,
    /// Sound volume (0.0 - 1.0)
    pub sound_volume: f32,
}

impl Default for OverlaySettings {
    fn default() -> Self {
        Self {
            enabled: true,
            opacity: 0.9,
            sound_enabled: true,
            sound_volume: 0.7,
        }
    }
}

/// Performance-related settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// Maximum CPU usage percentage
    pub max_cpu_percent: u32,
    /// Maximum memory usage in MB
    pub max_memory_mb: u32,
    /// Reduce activity when game is in menu/pause
    pub idle_optimization: bool,
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            max_cpu_percent: 10,
            max_memory_mb: 512,
            idle_optimization: true,
        }
    }
}

/// Load configuration from file
pub fn load_config(path: &Path) -> Result<AppConfig> {
    let content = std::fs::read_to_string(path)?;
    let config: AppConfig = toml::from_str(&content)?;
    Ok(config)
}

/// Save configuration to file
pub fn save_config(config: &AppConfig, path: &Path) -> Result<()> {
    let content = toml::to_string_pretty(config)?;
    std::fs::write(path, content)?;
    Ok(())
}
