//! Overlay view - Overlay customization and positioning

use egui::RichText;
use parking_lot::RwLock;
use std::sync::Arc;

use crate::dashboard::state::OverlayViewState;
use crate::dashboard::theme::{ThemeColors, color_with_alpha};
use crate::overlay::OverlayAnchor;
use crate::shared::SharedAppState;

/// Render the overlay view
pub fn render_overlay_view(
    ui: &mut egui::Ui,
    view_state: &mut OverlayViewState,
    shared_state: &Arc<RwLock<SharedAppState>>,
) {
    ui.heading(RichText::new("Overlay Settings").size(24.0).strong());
    ui.add_space(8.0);
    ui.label(
        RichText::new("Customize overlay appearance and positioning")
            .size(14.0)
            .color(ThemeColors::TEXT_SECONDARY)
    );

    ui.add_space(24.0);

    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.horizontal(|ui| {
            // Left column: Settings
            egui::Frame::none()
                .fill(ThemeColors::BG_MEDIUM)
                .rounding(egui::Rounding::same(8.0))
                .inner_margin(16.0)
                .show(ui, |ui| {
                    ui.set_min_width(320.0);

                    // Position Settings
                    ui.heading(RichText::new("Position").size(16.0));
                    ui.add_space(12.0);

                    {
                        let mut state = shared_state.write();

                        // Corner anchor selection
                        ui.label("Anchor corner:");
                        ui.add_space(4.0);

                        ui.horizontal(|ui| {
                            let anchors = [
                                (OverlayAnchor::TopLeft, "Top Left"),
                                (OverlayAnchor::TopRight, "Top Right"),
                            ];
                            for (anchor, label) in anchors {
                                if ui.selectable_label(state.overlay_config.anchor == anchor, label).clicked() {
                                    state.overlay_config.anchor = anchor;
                                }
                                ui.add_space(8.0);
                            }
                        });

                        ui.horizontal(|ui| {
                            let anchors = [
                                (OverlayAnchor::BottomLeft, "Bottom Left"),
                                (OverlayAnchor::BottomRight, "Bottom Right"),
                            ];
                            for (anchor, label) in anchors {
                                if ui.selectable_label(state.overlay_config.anchor == anchor, label).clicked() {
                                    state.overlay_config.anchor = anchor;
                                }
                                ui.add_space(8.0);
                            }
                        });

                        ui.add_space(12.0);

                        // Offset sliders
                        ui.horizontal(|ui| {
                            ui.label("Offset X:");
                            ui.add_space(8.0);
                            let mut offset_x = state.overlay_config.offset.0 as f32;
                            if ui.add(egui::Slider::new(&mut offset_x, 0.0..=200.0).suffix(" px")).changed() {
                                state.overlay_config.offset.0 = offset_x as i32;
                            }
                        });

                        ui.horizontal(|ui| {
                            ui.label("Offset Y:");
                            ui.add_space(8.0);
                            let mut offset_y = state.overlay_config.offset.1 as f32;
                            if ui.add(egui::Slider::new(&mut offset_y, 0.0..=200.0).suffix(" px")).changed() {
                                state.overlay_config.offset.1 = offset_y as i32;
                            }
                        });

                        ui.add_space(16.0);
                        ui.separator();
                        ui.add_space(16.0);

                        // Appearance Settings
                        ui.heading(RichText::new("Appearance").size(16.0));
                        ui.add_space(12.0);

                        // Opacity slider
                        ui.horizontal(|ui| {
                            ui.label("Opacity:");
                            ui.add_space(8.0);
                            let mut opacity = state.overlay_config.opacity;
                            if ui.add(egui::Slider::new(&mut opacity, 0.1..=1.0).show_value(true)).changed() {
                                state.overlay_config.opacity = opacity;
                                state.config.overlay.opacity = opacity;
                            }
                        });

                        ui.add_space(8.0);

                        // Max width slider
                        ui.horizontal(|ui| {
                            ui.label("Tip width:");
                            ui.add_space(8.0);
                            let mut max_width = state.overlay_config.max_width;
                            if ui.add(egui::Slider::new(&mut max_width, 200.0..=600.0).suffix(" px")).changed() {
                                state.overlay_config.max_width = max_width;
                            }
                        });

                        ui.add_space(8.0);

                        // Max tips slider
                        ui.horizontal(|ui| {
                            ui.label("Max tips shown:");
                            ui.add_space(8.0);
                            let mut max_tips = state.overlay_config.max_tips as f32;
                            if ui.add(egui::Slider::new(&mut max_tips, 1.0..=10.0)).changed() {
                                state.overlay_config.max_tips = max_tips as usize;
                            }
                        });

                        ui.add_space(8.0);

                        // Default duration slider
                        ui.horizontal(|ui| {
                            ui.label("Default duration:");
                            ui.add_space(8.0);
                            let mut duration = state.overlay_config.default_duration_ms as f32 / 1000.0;
                            if ui.add(egui::Slider::new(&mut duration, 1.0..=30.0).suffix(" s")).changed() {
                                state.overlay_config.default_duration_ms = (duration * 1000.0) as u64;
                            }
                        });

                        ui.add_space(16.0);
                        ui.separator();
                        ui.add_space(16.0);

                        // Monitor Settings
                        ui.heading(RichText::new("Display").size(16.0));
                        ui.add_space(12.0);

                        ui.horizontal(|ui| {
                            ui.label("Monitor:");
                            ui.add_space(8.0);
                            let mut monitor = state.overlay_config.monitor_index.unwrap_or(0) as f32;
                            if ui.add(egui::Slider::new(&mut monitor, 0.0..=3.0)).changed() {
                                state.overlay_config.monitor_index = Some(monitor as usize);
                            }
                        });

                        ui.horizontal(|ui| {
                            ui.label("Overlay enabled:");
                            ui.add_space(8.0);
                            ui.checkbox(&mut state.overlay_config.enabled, "");
                        });
                    }
                });

            ui.add_space(16.0);

            // Right column: Preview
            egui::Frame::none()
                .fill(ThemeColors::BG_MEDIUM)
                .rounding(egui::Rounding::same(8.0))
                .inner_margin(16.0)
                .show(ui, |ui| {
                    ui.set_min_width(300.0);

                    ui.heading(RichText::new("Preview").size(16.0));
                    ui.add_space(12.0);

                    // Monitor preview area
                    egui::Frame::none()
                        .fill(ThemeColors::BG_DARK)
                        .rounding(egui::Rounding::same(6.0))
                        .stroke(egui::Stroke::new(1.0, ThemeColors::BORDER))
                        .show(ui, |ui| {
                            let available_size = ui.available_size();
                            let preview_size = egui::vec2(
                                available_size.x.min(280.0),
                                160.0
                            );
                            ui.set_min_size(preview_size);

                            // Draw a visual representation of overlay position
                            let rect = ui.available_rect_before_wrap();
                            let painter = ui.painter();

                            // Draw "monitor" outline
                            painter.rect_stroke(
                                rect.shrink(4.0),
                                egui::Rounding::same(4.0),
                                egui::Stroke::new(1.0, ThemeColors::TEXT_MUTED),
                            );

                            // Draw overlay position indicator
                            let state = shared_state.read();
                            let anchor = state.overlay_config.anchor;
                            let offset = state.overlay_config.offset;

                            let tip_size = egui::vec2(80.0, 30.0);
                            let scale = 0.15; // Scale offset for preview

                            let tip_pos = match anchor {
                                OverlayAnchor::TopLeft => {
                                    rect.left_top() + egui::vec2(8.0 + offset.0 as f32 * scale, 8.0 + offset.1 as f32 * scale)
                                }
                                OverlayAnchor::TopRight => {
                                    rect.right_top() + egui::vec2(-8.0 - tip_size.x - offset.0 as f32 * scale, 8.0 + offset.1 as f32 * scale)
                                }
                                OverlayAnchor::BottomLeft => {
                                    rect.left_bottom() + egui::vec2(8.0 + offset.0 as f32 * scale, -8.0 - tip_size.y - offset.1 as f32 * scale)
                                }
                                OverlayAnchor::BottomRight => {
                                    rect.right_bottom() + egui::vec2(-8.0 - tip_size.x - offset.0 as f32 * scale, -8.0 - tip_size.y - offset.1 as f32 * scale)
                                }
                            };

                            let tip_rect = egui::Rect::from_min_size(tip_pos, tip_size);
                            let opacity = state.overlay_config.opacity;

                            let alpha = (opacity * 255.0) as u8;
                            painter.rect_filled(
                                tip_rect,
                                egui::Rounding::same(4.0),
                                color_with_alpha(ThemeColors::ACCENT_PRIMARY, alpha),
                            );

                            painter.text(
                                tip_rect.center(),
                                egui::Align2::CENTER_CENTER,
                                "Tip",
                                egui::FontId::proportional(10.0),
                                color_with_alpha(egui::Color32::WHITE, alpha),
                            );
                        });

                    ui.add_space(16.0);

                    // Test tip section
                    ui.heading(RichText::new("Test Tip").size(14.0));
                    ui.add_space(8.0);

                    ui.horizontal(|ui| {
                        ui.label("Message:");
                        ui.text_edit_singleline(&mut view_state.preview_tip_text);
                    });

                    ui.horizontal(|ui| {
                        ui.label("Priority:");
                        let mut priority = view_state.preview_tip_priority as f32;
                        ui.add(egui::Slider::new(&mut priority, 0.0..=100.0));
                        view_state.preview_tip_priority = priority as u32;
                    });

                    ui.add_space(8.0);

                    if ui.button("Send Test Tip").clicked() {
                        // Will be connected to tip sender
                    }
                });
        });
    });
}
