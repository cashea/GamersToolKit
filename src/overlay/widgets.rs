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
