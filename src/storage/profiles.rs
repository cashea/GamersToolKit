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
                    bounds: (0.1, 0.9, 0.1, 0.05),
                    content_type: ContentType::Number,
                },
                OcrRegion {
                    id: "mana".to_string(),
                    bounds: (0.2, 0.9, 0.1, 0.05),
                    content_type: ContentType::Percentage,
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
            bounds: (0.5, 0.5, 0.2, 0.1),
            content_type: ContentType::Text,
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
        };

        let json = serde_json::to_string(&profile).unwrap();
        let parsed: GameProfile = serde_json::from_str(&json).unwrap();

        assert!(parsed.executables.is_empty());
        assert!(parsed.ocr_regions.is_empty());
        assert!(parsed.templates.is_empty());
        assert!(parsed.rules.is_empty());
        assert!(parsed.labeled_regions.is_empty());
    }
}
