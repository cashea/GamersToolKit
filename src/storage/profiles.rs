//! Game profile storage and loading

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// A game profile definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameProfile {
    /// Profile identifier
    pub id: String,
    /// Display name
    pub name: String,
    /// Game executable name(s) for auto-detection
    pub executables: Vec<String>,
    /// Profile version
    pub version: String,
    /// OCR regions to monitor
    pub ocr_regions: Vec<OcrRegion>,
    /// Visual templates to detect
    pub templates: Vec<TemplateDefinition>,
    /// Rules to apply
    pub rules: Vec<RuleDefinition>,
}

/// A region to run OCR on
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrRegion {
    /// Region identifier
    pub id: String,
    /// Region bounds (x, y, width, height) as percentages of screen
    pub bounds: (f32, f32, f32, f32),
    /// Expected content type
    pub content_type: ContentType,
}

/// Type of content expected in a region
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContentType {
    Text,
    Number,
    Percentage,
    Time,
}

/// A visual template definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateDefinition {
    /// Template identifier
    pub id: String,
    /// Path to template image
    pub image_path: String,
    /// Match threshold
    pub threshold: f32,
}

/// A rule definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleDefinition {
    /// Rule identifier
    pub id: String,
    /// Rule name
    pub name: String,
    /// Whether enabled by default
    pub enabled: bool,
    /// Rhai script code
    pub script: String,
}

/// Load a game profile from file
pub fn load_profile(path: &Path) -> Result<GameProfile> {
    let content = std::fs::read_to_string(path)?;
    let profile: GameProfile = serde_json::from_str(&content)?;
    Ok(profile)
}

/// Save a game profile to file
pub fn save_profile(profile: &GameProfile, path: &Path) -> Result<()> {
    let content = serde_json::to_string_pretty(profile)?;
    std::fs::write(path, content)?;
    Ok(())
}
