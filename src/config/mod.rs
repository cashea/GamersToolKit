//! Application Configuration
//!
//! User settings and preferences stored in TOML format.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

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
    /// Target window title (partial match) or empty for primary monitor
    pub target_window: Option<String>,
    /// Maximum capture FPS
    pub max_fps: u32,
    /// Capture cursor in frames
    pub capture_cursor: bool,
    /// Draw border around captured window
    pub draw_border: bool,
}

impl Default for CaptureSettings {
    fn default() -> Self {
        Self {
            target_window: None,
            max_fps: 30,
            capture_cursor: false,
            draw_border: false,
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_default_app_config() {
        let config = AppConfig::default();

        // Check general defaults
        assert!(!config.general.start_minimized);
        assert!(!config.general.auto_start);
        assert!(config.general.check_updates);

        // Check capture defaults
        assert!(config.capture.target_window.is_none());
        assert_eq!(config.capture.max_fps, 30);
        assert!(!config.capture.capture_cursor);
        assert!(!config.capture.draw_border);

        // Check overlay defaults
        assert!(config.overlay.enabled);
        assert!((config.overlay.opacity - 0.9).abs() < 0.01);
        assert!(config.overlay.sound_enabled);
        assert!((config.overlay.sound_volume - 0.7).abs() < 0.01);

        // Check performance defaults
        assert_eq!(config.performance.max_cpu_percent, 10);
        assert_eq!(config.performance.max_memory_mb, 512);
        assert!(config.performance.idle_optimization);
    }

    #[test]
    fn test_config_serialization_roundtrip() {
        let config = AppConfig::default();

        // Serialize to TOML
        let toml_str = toml::to_string_pretty(&config).unwrap();

        // Deserialize back
        let parsed: AppConfig = toml::from_str(&toml_str).unwrap();

        // Verify values match
        assert_eq!(config.general.start_minimized, parsed.general.start_minimized);
        assert_eq!(config.capture.max_fps, parsed.capture.max_fps);
        assert_eq!(config.overlay.enabled, parsed.overlay.enabled);
        assert_eq!(config.performance.max_memory_mb, parsed.performance.max_memory_mb);
    }

    #[test]
    fn test_config_with_custom_values() {
        let mut config = AppConfig::default();
        config.capture.target_window = Some("My Game".to_string());
        config.capture.max_fps = 60;
        config.overlay.opacity = 0.5;

        // Serialize and deserialize
        let toml_str = toml::to_string_pretty(&config).unwrap();
        let parsed: AppConfig = toml::from_str(&toml_str).unwrap();

        assert_eq!(parsed.capture.target_window, Some("My Game".to_string()));
        assert_eq!(parsed.capture.max_fps, 60);
        assert!((parsed.overlay.opacity - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_save_and_load_config() {
        let config = AppConfig::default();

        // Create a temporary file
        let mut temp_file = NamedTempFile::new().unwrap();

        // Save config
        save_config(&config, temp_file.path()).unwrap();

        // Load config
        let loaded = load_config(temp_file.path()).unwrap();

        // Verify
        assert_eq!(config.general.check_updates, loaded.general.check_updates);
        assert_eq!(config.capture.max_fps, loaded.capture.max_fps);
    }

    #[test]
    fn test_load_config_file_not_found() {
        let result = load_config(Path::new("/nonexistent/path/config.toml"));
        assert!(result.is_err());
    }

    #[test]
    fn test_load_config_invalid_toml() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "this is not valid toml {{{{").unwrap();

        let result = load_config(temp_file.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_capture_settings_clone() {
        let settings = CaptureSettings {
            target_window: Some("Test".to_string()),
            max_fps: 60,
            capture_cursor: true,
            draw_border: true,
        };

        let cloned = settings.clone();
        assert_eq!(settings.target_window, cloned.target_window);
        assert_eq!(settings.max_fps, cloned.max_fps);
    }
}
