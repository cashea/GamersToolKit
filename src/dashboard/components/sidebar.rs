//! Sidebar navigation component

use egui::{Color32, RichText, Rounding, Sense, Vec2};
use crate::dashboard::state::DashboardView;
use crate::dashboard::theme::{ThemeColors, color_with_alpha};

/// Render the sidebar navigation
pub fn render_sidebar(ui: &mut egui::Ui, current_view: &mut DashboardView) {
    ui.vertical(|ui| {
        ui.add_space(16.0);

        // Logo/Title
        ui.horizontal(|ui| {
            ui.add_space(12.0);
            ui.label(
                RichText::new("GTK")
                    .size(24.0)
                    .color(ThemeColors::ACCENT_PRIMARY)
                    .strong()
            );
        });

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.add_space(12.0);
            ui.label(
                RichText::new("GamersToolKit")
                    .size(11.0)
                    .color(ThemeColors::TEXT_MUTED)
            );
        });

        ui.add_space(24.0);
        ui.separator();
        ui.add_space(16.0);

        // Navigation items
        for view in [
            DashboardView::Home,
            DashboardView::Capture,
            DashboardView::Overlay,
            DashboardView::Vision,
            DashboardView::Screens,
            DashboardView::Profiles,
            DashboardView::Settings,
        ] {
            let is_selected = *current_view == view;
            if nav_button(ui, view.icon(), view.name(), is_selected) {
                *current_view = view;
            }
            ui.add_space(4.0);
        }

        // Spacer to push version to bottom
        ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                ui.add_space(12.0);
                ui.label(
                    RichText::new("v0.1.0")
                        .size(10.0)
                        .color(ThemeColors::TEXT_MUTED)
                );
            });
            ui.add_space(8.0);
            ui.separator();
        });
    });
}

/// Render a navigation button
fn nav_button(ui: &mut egui::Ui, icon: &str, label: &str, is_selected: bool) -> bool {
    let available_width = ui.available_width();
    let desired_size = Vec2::new(available_width - 16.0, 36.0);

    let (rect, response) = ui.allocate_exact_size(desired_size, Sense::click());

    if ui.is_rect_visible(rect) {
        let bg_color = if is_selected {
            color_with_alpha(ThemeColors::ACCENT_PRIMARY, 51) // ~0.2 alpha
        } else if response.hovered() {
            ThemeColors::BG_HOVER
        } else {
            Color32::TRANSPARENT
        };

        let text_color = if is_selected {
            ThemeColors::ACCENT_PRIMARY
        } else if response.hovered() {
            ThemeColors::TEXT_PRIMARY
        } else {
            ThemeColors::TEXT_SECONDARY
        };

        // Draw background
        ui.painter().rect_filled(
            rect.shrink2(Vec2::new(8.0, 0.0)),
            Rounding::same(6.0),
            bg_color,
        );

        // Draw selection indicator
        if is_selected {
            let indicator_rect = egui::Rect::from_min_size(
                rect.left_top() + Vec2::new(8.0, 6.0),
                Vec2::new(3.0, rect.height() - 12.0),
            );
            ui.painter().rect_filled(
                indicator_rect,
                Rounding::same(1.5),
                ThemeColors::ACCENT_PRIMARY,
            );
        }

        // Draw icon
        let icon_pos = rect.left_center() + Vec2::new(24.0, 0.0);
        ui.painter().text(
            icon_pos,
            egui::Align2::LEFT_CENTER,
            icon,
            egui::FontId::proportional(14.0),
            text_color,
        );

        // Draw label
        let label_pos = rect.left_center() + Vec2::new(48.0, 0.0);
        ui.painter().text(
            label_pos,
            egui::Align2::LEFT_CENTER,
            label,
            egui::FontId::proportional(14.0),
            text_color,
        );
    }

    response.clicked()
}
