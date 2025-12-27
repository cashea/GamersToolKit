//! Status card component for displaying status information

use egui::{Color32, RichText, Rounding, Vec2};
use crate::dashboard::theme::ThemeColors;

/// A card displaying status information
pub struct StatusCard {
    pub title: String,
    pub value: String,
    pub status: CardStatus,
    pub icon: Option<String>,
}

/// Status types for cards
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CardStatus {
    Active,
    Inactive,
    Warning,
    Error,
}

impl CardStatus {
    pub fn color(&self) -> Color32 {
        match self {
            CardStatus::Active => ThemeColors::STATUS_RUNNING,
            CardStatus::Inactive => ThemeColors::STATUS_STOPPED,
            CardStatus::Warning => ThemeColors::ACCENT_WARNING,
            CardStatus::Error => ThemeColors::STATUS_ERROR,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            CardStatus::Active => "Active",
            CardStatus::Inactive => "Inactive",
            CardStatus::Warning => "Warning",
            CardStatus::Error => "Error",
        }
    }
}

impl StatusCard {
    pub fn new(title: impl Into<String>, value: impl Into<String>, status: CardStatus) -> Self {
        Self {
            title: title.into(),
            value: value.into(),
            status,
            icon: None,
        }
    }

    pub fn with_icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    pub fn show(&self, ui: &mut egui::Ui) {
        egui::Frame::none()
            .fill(ThemeColors::BG_MEDIUM)
            .rounding(Rounding::same(8.0))
            .inner_margin(16.0)
            .show(ui, |ui| {
                ui.set_min_width(180.0);

                ui.horizontal(|ui| {
                    // Status indicator dot
                    let dot_rect = egui::Rect::from_center_size(
                        ui.cursor().left_top() + Vec2::new(6.0, 10.0),
                        Vec2::splat(8.0),
                    );
                    ui.painter().circle_filled(
                        dot_rect.center(),
                        4.0,
                        self.status.color(),
                    );
                    ui.add_space(16.0);

                    ui.vertical(|ui| {
                        // Title
                        ui.label(
                            RichText::new(&self.title)
                                .size(12.0)
                                .color(ThemeColors::TEXT_MUTED)
                        );

                        ui.add_space(4.0);

                        // Value
                        ui.label(
                            RichText::new(&self.value)
                                .size(18.0)
                                .color(ThemeColors::TEXT_PRIMARY)
                                .strong()
                        );

                        ui.add_space(4.0);

                        // Status label
                        ui.label(
                            RichText::new(self.status.label())
                                .size(11.0)
                                .color(self.status.color())
                        );
                    });
                });
            });
    }
}
