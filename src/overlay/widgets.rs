//! Custom egui widgets for the overlay

/// Style configuration for tip widgets
#[derive(Debug, Clone)]
pub struct TipStyle {
    /// Background color (RGBA)
    pub background: [f32; 4],
    /// Text color (RGBA)
    pub text_color: [f32; 4],
    /// Border radius
    pub corner_radius: f32,
    /// Padding
    pub padding: f32,
}

impl Default for TipStyle {
    fn default() -> Self {
        Self {
            background: [0.1, 0.1, 0.1, 0.85],
            text_color: [1.0, 1.0, 1.0, 1.0],
            corner_radius: 8.0,
            padding: 12.0,
        }
    }
}

/// Priority-based style overrides
#[derive(Debug, Clone)]
pub struct PriorityStyles {
    pub low: TipStyle,
    pub medium: TipStyle,
    pub high: TipStyle,
    pub critical: TipStyle,
}

impl Default for PriorityStyles {
    fn default() -> Self {
        Self {
            low: TipStyle {
                background: [0.2, 0.2, 0.3, 0.8],
                ..Default::default()
            },
            medium: TipStyle::default(),
            high: TipStyle {
                background: [0.4, 0.3, 0.1, 0.9],
                text_color: [1.0, 0.9, 0.6, 1.0],
                ..Default::default()
            },
            critical: TipStyle {
                background: [0.5, 0.1, 0.1, 0.95],
                text_color: [1.0, 0.8, 0.8, 1.0],
                ..Default::default()
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tip_style_default() {
        let style = TipStyle::default();

        // Background should be dark with high alpha
        assert!((style.background[0] - 0.1).abs() < 0.01);
        assert!((style.background[3] - 0.85).abs() < 0.01);

        // Text should be white
        assert!((style.text_color[0] - 1.0).abs() < 0.01);
        assert!((style.text_color[1] - 1.0).abs() < 0.01);
        assert!((style.text_color[2] - 1.0).abs() < 0.01);

        // Corner radius and padding
        assert!((style.corner_radius - 8.0).abs() < 0.01);
        assert!((style.padding - 12.0).abs() < 0.01);
    }

    #[test]
    fn test_tip_style_clone() {
        let style = TipStyle {
            background: [0.5, 0.5, 0.5, 1.0],
            text_color: [0.0, 0.0, 0.0, 1.0],
            corner_radius: 10.0,
            padding: 20.0,
        };

        let cloned = style.clone();
        assert_eq!(style.background, cloned.background);
        assert_eq!(style.text_color, cloned.text_color);
        assert_eq!(style.corner_radius, cloned.corner_radius);
        assert_eq!(style.padding, cloned.padding);
    }

    #[test]
    fn test_priority_styles_default() {
        let styles = PriorityStyles::default();

        // Low priority - blueish background
        assert!(styles.low.background[2] > styles.low.background[0]); // More blue than red

        // Medium priority - default style
        assert!((styles.medium.background[0] - 0.1).abs() < 0.01);

        // High priority - yellowish (more red and green)
        assert!(styles.high.background[0] > styles.high.background[2]); // More red than blue
        assert!(styles.high.text_color[1] > 0.8); // Yellowish text

        // Critical priority - reddish background
        assert!(styles.critical.background[0] > styles.critical.background[1]); // More red than green
        assert!(styles.critical.background[0] > styles.critical.background[2]); // More red than blue
    }

    #[test]
    fn test_priority_styles_clone() {
        let styles = PriorityStyles::default();
        let cloned = styles.clone();

        assert_eq!(styles.low.background, cloned.low.background);
        assert_eq!(styles.high.text_color, cloned.high.text_color);
        assert_eq!(styles.critical.corner_radius, cloned.critical.corner_radius);
    }

    #[test]
    fn test_color_values_in_range() {
        let styles = PriorityStyles::default();

        // Helper to check all colors are in valid range [0.0, 1.0]
        let check_color = |color: &[f32; 4]| {
            for &c in color {
                assert!(c >= 0.0 && c <= 1.0, "Color component out of range: {}", c);
            }
        };

        check_color(&styles.low.background);
        check_color(&styles.low.text_color);
        check_color(&styles.medium.background);
        check_color(&styles.medium.text_color);
        check_color(&styles.high.background);
        check_color(&styles.high.text_color);
        check_color(&styles.critical.background);
        check_color(&styles.critical.text_color);
    }

    #[test]
    fn test_custom_tip_style() {
        let custom = TipStyle {
            background: [1.0, 0.0, 0.0, 0.5],
            text_color: [0.0, 1.0, 0.0, 1.0],
            corner_radius: 0.0,
            padding: 5.0,
        };

        assert_eq!(custom.background[0], 1.0); // Red background
        assert_eq!(custom.text_color[1], 1.0); // Green text
        assert_eq!(custom.corner_radius, 0.0); // No rounding
    }
}
