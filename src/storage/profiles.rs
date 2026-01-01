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
    /// User-defined labeled regions from vision/OCR
    #[serde(default)]
    pub labeled_regions: Vec<LabeledRegion>,
    /// Screen definitions for screen recognition
    #[serde(default)]
    pub screens: Vec<ScreenDefinition>,
    /// Whether screen recognition is enabled for this profile
    #[serde(default)]
    pub screen_recognition_enabled: bool,
    /// Interval between screen recognition checks in milliseconds
    #[serde(default = "default_screen_check_interval")]
    pub screen_check_interval_ms: u32,
}

fn default_screen_check_interval() -> u32 {
    500
}

/// A labeled region that maps detected text to a user-defined name
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabeledRegion {
    /// User-defined label (e.g., "Parsteel", "Tritanium")
    pub label: String,
    /// The text that was detected in this region
    pub matched_text: String,
    /// Bounding box (x, y, width, height) in pixels
    pub bounds: (u32, u32, u32, u32),
    /// Confidence score from OCR
    pub confidence: f32,
}

/// A region to run OCR on (Zone OCR)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrRegion {
    /// Region identifier (unique within profile)
    pub id: String,
    /// User-friendly name for this zone
    #[serde(default)]
    pub name: String,
    /// Region bounds (x, y, width, height) as percentages of screen (0.0-1.0)
    pub bounds: (f32, f32, f32, f32),
    /// Expected content type
    pub content_type: ContentType,
    /// Whether this zone is enabled for OCR
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Zone-specific OCR preprocessing settings (uses global settings if None)
    #[serde(default)]
    pub preprocessing: Option<crate::config::OcrPreprocessing>,
}

fn default_true() -> bool {
    true
}

/// Type of content expected in a region
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContentType {
    #[default]
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

// ============================================================================
// Screen Recognition Types
// ============================================================================

/// A screen that can be recognized within a game
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenDefinition {
    /// Unique identifier for this screen
    pub id: String,
    /// Display name (e.g., "Main Menu", "Inventory")
    pub name: String,
    /// Parent screen ID for hierarchical organization (None = root level)
    #[serde(default)]
    pub parent_id: Option<String>,
    /// How to match this screen
    pub match_mode: ScreenMatchMode,
    /// Anchor regions for anchor-based matching
    #[serde(default)]
    pub anchors: Vec<ScreenAnchor>,
    /// Full screenshot template for full-screen matching
    #[serde(default)]
    pub full_template: Option<ScreenTemplate>,
    /// Minimum confidence threshold for a match (0.0-1.0)
    #[serde(default = "default_match_threshold")]
    pub match_threshold: f32,
    /// Whether this screen is enabled for recognition
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Priority for matching order (higher = checked first)
    #[serde(default)]
    pub priority: u32,
    /// OCR zone overrides when this screen is active
    #[serde(default)]
    pub ocr_zone_overrides: Vec<ZoneOverride>,
    /// Rule IDs to trigger when entering this screen
    #[serde(default)]
    pub rules_to_trigger: Vec<String>,
    /// Whether to show overlay notification on screen detection
    #[serde(default = "default_true")]
    pub show_notification: bool,
}

fn default_match_threshold() -> f32 {
    0.8
}

/// How to match a screen
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScreenMatchMode {
    /// Match using a full screenshot template
    FullScreenshot,
    /// Match using anchor regions (visual or text)
    #[default]
    Anchors,
}

/// An anchor region for screen recognition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenAnchor {
    /// Unique identifier for this anchor
    pub id: String,
    /// Type of anchor (visual template or text OCR)
    pub anchor_type: AnchorType,
    /// Region bounds (x, y, width, height) as percentages of screen (0.0-1.0)
    pub bounds: (f32, f32, f32, f32),
    /// Template image data for visual anchors (PNG encoded)
    #[serde(default)]
    pub template_data: Option<Vec<u8>>,
    /// Expected text for text anchors
    #[serde(default)]
    pub expected_text: Option<String>,
    /// Similarity threshold for fuzzy text matching (0.0-1.0)
    #[serde(default = "default_text_similarity")]
    pub text_similarity: f32,
    /// Whether this anchor must match (vs optional/bonus)
    #[serde(default = "default_true")]
    pub required: bool,
}

fn default_text_similarity() -> f32 {
    0.8
}

/// Type of anchor for screen matching
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnchorType {
    /// Visual template matching
    #[default]
    Visual,
    /// Text OCR matching
    Text,
}

/// A full screen template for matching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenTemplate {
    /// PNG-encoded image data
    pub image_data: Vec<u8>,
    /// Original image width
    pub width: u32,
    /// Original image height
    pub height: u32,
    /// When this template was captured (ISO 8601 timestamp)
    pub captured_at: String,
}

/// Override for an OCR zone when a screen is active
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZoneOverride {
    /// ID of the OCR zone to override
    pub zone_id: String,
    /// Whether the zone should be enabled
    pub enabled: bool,
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

/// Load all game profiles from a directory
pub fn load_all_profiles(dir: &Path) -> Result<Vec<GameProfile>> {
    let mut profiles = Vec::new();

    if !dir.exists() {
        return Ok(profiles);
    }

    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        // Only load .json files
        if path.extension().map_or(false, |ext| ext == "json") {
            match load_profile(&path) {
                Ok(profile) => {
                    profiles.push(profile);
                }
                Err(e) => {
                    tracing::warn!("Failed to load profile from {:?}: {}", path, e);
                }
            }
        }
    }

    Ok(profiles)
}

/// Delete a game profile file
pub fn delete_profile(dir: &Path, profile_id: &str) -> Result<()> {
    let profile_path = dir.join(format!("{}.json", profile_id));
    if profile_path.exists() {
        std::fs::remove_file(&profile_path)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    fn create_test_profile() -> GameProfile {
        GameProfile {
            id: "test-game".to_string(),
            name: "Test Game".to_string(),
            executables: vec!["testgame.exe".to_string(), "testgame64.exe".to_string()],
            version: "1.0.0".to_string(),
            ocr_regions: vec![
                OcrRegion {
                    id: "health".to_string(),
                    name: "Health".to_string(),
                    bounds: (0.1, 0.9, 0.1, 0.05),
                    content_type: ContentType::Number,
                    enabled: true,
                    preprocessing: None,
                },
                OcrRegion {
                    id: "mana".to_string(),
                    name: "Mana".to_string(),
                    bounds: (0.2, 0.9, 0.1, 0.05),
                    content_type: ContentType::Percentage,
                    enabled: true,
                    preprocessing: None,
                },
            ],
            templates: vec![
                TemplateDefinition {
                    id: "low_health_icon".to_string(),
                    image_path: "templates/low_health.png".to_string(),
                    threshold: 0.8,
                },
            ],
            rules: vec![
                RuleDefinition {
                    id: "low_health_warning".to_string(),
                    name: "Low Health Warning".to_string(),
                    enabled: true,
                    script: r#"if health < 20 { alert("Low health!") }"#.to_string(),
                },
            ],
            labeled_regions: vec![
                LabeledRegion {
                    label: "Gold".to_string(),
                    matched_text: "1,234".to_string(),
                    bounds: (100, 50, 80, 20),
                    confidence: 0.95,
                },
            ],
            screens: vec![
                ScreenDefinition {
                    id: "main_menu".to_string(),
                    name: "Main Menu".to_string(),
                    parent_id: None,
                    match_mode: ScreenMatchMode::Anchors,
                    anchors: vec![
                        ScreenAnchor {
                            id: "title_text".to_string(),
                            anchor_type: AnchorType::Text,
                            bounds: (0.4, 0.1, 0.2, 0.1),
                            template_data: None,
                            expected_text: Some("Test Game".to_string()),
                            text_similarity: 0.9,
                            required: true,
                        },
                    ],
                    full_template: None,
                    match_threshold: 0.8,
                    enabled: true,
                    priority: 10,
                    ocr_zone_overrides: vec![],
                    rules_to_trigger: vec![],
                    show_notification: true,
                },
            ],
            screen_recognition_enabled: true,
            screen_check_interval_ms: 500,
        }
    }

    #[test]
    fn test_profile_serialization_roundtrip() {
        let profile = create_test_profile();

        // Serialize to JSON
        let json = serde_json::to_string_pretty(&profile).unwrap();

        // Deserialize back
        let parsed: GameProfile = serde_json::from_str(&json).unwrap();

        assert_eq!(profile.id, parsed.id);
        assert_eq!(profile.name, parsed.name);
        assert_eq!(profile.executables.len(), parsed.executables.len());
        assert_eq!(profile.ocr_regions.len(), parsed.ocr_regions.len());
        assert_eq!(profile.templates.len(), parsed.templates.len());
        assert_eq!(profile.rules.len(), parsed.rules.len());
        assert_eq!(profile.labeled_regions.len(), parsed.labeled_regions.len());
        assert_eq!(profile.screens.len(), parsed.screens.len());
        assert_eq!(profile.screen_recognition_enabled, parsed.screen_recognition_enabled);
        assert_eq!(profile.screen_check_interval_ms, parsed.screen_check_interval_ms);
    }

    #[test]
    fn test_save_and_load_profile() {
        let profile = create_test_profile();

        // Create temp file
        let temp_file = NamedTempFile::new().unwrap();

        // Save profile
        save_profile(&profile, temp_file.path()).unwrap();

        // Load profile
        let loaded = load_profile(temp_file.path()).unwrap();

        assert_eq!(profile.id, loaded.id);
        assert_eq!(profile.name, loaded.name);
        assert_eq!(profile.version, loaded.version);
    }

    #[test]
    fn test_ocr_region_bounds() {
        let region = OcrRegion {
            id: "test".to_string(),
            name: "Test Zone".to_string(),
            bounds: (0.5, 0.5, 0.2, 0.1),
            content_type: ContentType::Text,
            enabled: true,
            preprocessing: None,
        };

        assert_eq!(region.bounds.0, 0.5); // x
        assert_eq!(region.bounds.1, 0.5); // y
        assert_eq!(region.bounds.2, 0.2); // width
        assert_eq!(region.bounds.3, 0.1); // height
    }

    #[test]
    fn test_content_type_serialization() {
        let types = vec![
            ContentType::Text,
            ContentType::Number,
            ContentType::Percentage,
            ContentType::Time,
        ];

        for content_type in types {
            let json = serde_json::to_string(&content_type).unwrap();
            let parsed: ContentType = serde_json::from_str(&json).unwrap();

            // Compare debug representations
            assert_eq!(format!("{:?}", content_type), format!("{:?}", parsed));
        }
    }

    #[test]
    fn test_template_definition() {
        let template = TemplateDefinition {
            id: "test_icon".to_string(),
            image_path: "icons/test.png".to_string(),
            threshold: 0.85,
        };

        let json = serde_json::to_string(&template).unwrap();
        let parsed: TemplateDefinition = serde_json::from_str(&json).unwrap();

        assert_eq!(template.id, parsed.id);
        assert_eq!(template.image_path, parsed.image_path);
        assert!((template.threshold - parsed.threshold).abs() < 0.001);
    }

    #[test]
    fn test_rule_definition() {
        let rule = RuleDefinition {
            id: "test_rule".to_string(),
            name: "Test Rule".to_string(),
            enabled: false,
            script: "print(\"hello\")".to_string(),
        };

        let json = serde_json::to_string(&rule).unwrap();
        let parsed: RuleDefinition = serde_json::from_str(&json).unwrap();

        assert_eq!(rule.id, parsed.id);
        assert_eq!(rule.name, parsed.name);
        assert_eq!(rule.enabled, parsed.enabled);
        assert_eq!(rule.script, parsed.script);
    }

    #[test]
    fn test_load_profile_file_not_found() {
        let result = load_profile(Path::new("/nonexistent/profile.json"));
        assert!(result.is_err());
    }

    #[test]
    fn test_load_profile_invalid_json() {
        let mut temp_file = NamedTempFile::new().unwrap();
        std::io::Write::write_all(&mut temp_file, b"not valid json {{{").unwrap();

        let result = load_profile(temp_file.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_profile_clone() {
        let profile = create_test_profile();
        let cloned = profile.clone();

        assert_eq!(profile.id, cloned.id);
        assert_eq!(profile.ocr_regions.len(), cloned.ocr_regions.len());
    }

    #[test]
    fn test_empty_profile() {
        let profile = GameProfile {
            id: "empty".to_string(),
            name: "Empty Profile".to_string(),
            executables: vec![],
            version: "0.1.0".to_string(),
            ocr_regions: vec![],
            templates: vec![],
            rules: vec![],
            labeled_regions: vec![],
            screens: vec![],
            screen_recognition_enabled: false,
            screen_check_interval_ms: 500,
        };

        let json = serde_json::to_string(&profile).unwrap();
        let parsed: GameProfile = serde_json::from_str(&json).unwrap();

        assert!(parsed.executables.is_empty());
        assert!(parsed.ocr_regions.is_empty());
        assert!(parsed.templates.is_empty());
        assert!(parsed.rules.is_empty());
        assert!(parsed.labeled_regions.is_empty());
        assert!(parsed.screens.is_empty());
        assert!(!parsed.screen_recognition_enabled);
    }

    #[test]
    fn test_screen_definition_serialization() {
        let screen = ScreenDefinition {
            id: "inventory".to_string(),
            name: "Inventory Screen".to_string(),
            parent_id: Some("in_game".to_string()),
            match_mode: ScreenMatchMode::Anchors,
            anchors: vec![
                ScreenAnchor {
                    id: "header".to_string(),
                    anchor_type: AnchorType::Text,
                    bounds: (0.1, 0.05, 0.3, 0.1),
                    template_data: None,
                    expected_text: Some("INVENTORY".to_string()),
                    text_similarity: 0.85,
                    required: true,
                },
                ScreenAnchor {
                    id: "icon".to_string(),
                    anchor_type: AnchorType::Visual,
                    bounds: (0.05, 0.05, 0.05, 0.05),
                    template_data: Some(vec![1, 2, 3, 4]), // Dummy PNG data
                    expected_text: None,
                    text_similarity: 0.8,
                    required: false,
                },
            ],
            full_template: None,
            match_threshold: 0.75,
            enabled: true,
            priority: 5,
            ocr_zone_overrides: vec![
                ZoneOverride {
                    zone_id: "gold".to_string(),
                    enabled: true,
                },
            ],
            rules_to_trigger: vec!["inventory_opened".to_string()],
            show_notification: false,
        };

        let json = serde_json::to_string_pretty(&screen).unwrap();
        let parsed: ScreenDefinition = serde_json::from_str(&json).unwrap();

        assert_eq!(screen.id, parsed.id);
        assert_eq!(screen.name, parsed.name);
        assert_eq!(screen.parent_id, parsed.parent_id);
        assert_eq!(screen.match_mode, parsed.match_mode);
        assert_eq!(screen.anchors.len(), parsed.anchors.len());
        assert_eq!(screen.ocr_zone_overrides.len(), parsed.ocr_zone_overrides.len());
        assert_eq!(screen.rules_to_trigger.len(), parsed.rules_to_trigger.len());
    }

    #[test]
    fn test_screen_match_mode_serialization() {
        let modes = vec![ScreenMatchMode::FullScreenshot, ScreenMatchMode::Anchors];

        for mode in modes {
            let json = serde_json::to_string(&mode).unwrap();
            let parsed: ScreenMatchMode = serde_json::from_str(&json).unwrap();
            assert_eq!(mode, parsed);
        }
    }

    #[test]
    fn test_anchor_type_serialization() {
        let types = vec![AnchorType::Visual, AnchorType::Text];

        for anchor_type in types {
            let json = serde_json::to_string(&anchor_type).unwrap();
            let parsed: AnchorType = serde_json::from_str(&json).unwrap();
            assert_eq!(anchor_type, parsed);
        }
    }
}
