//! Zone OCR management view
//!
//! Dashboard panel for creating, editing, and managing OCR zones.

use egui::{Color32, RichText, Rounding, Stroke, Vec2};
use uuid::Uuid;

use crate::dashboard::state::{VisionViewState, ZoneOcrResult};
use crate::storage::profiles::{ContentType, OcrRegion};

/// Render the zone OCR management panel
pub fn render_zone_ocr_panel(
    ui: &mut egui::Ui,
    view_state: &mut VisionViewState,
    max_height: f32,
) {
    ui.vertical(|ui| {
        // Header with add button
        ui.horizontal(|ui| {
            ui.heading("Zone OCR");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Disable button while selecting
                let is_selecting = view_state.zone_selection.is_selecting;
                if ui.add_enabled(!is_selecting, egui::Button::new("+ Create Zone")).clicked() {
                    view_state.pending_zone_selection_mode = true;
                }
            });
        });

        ui.separator();

        // Show zone selection status
        if view_state.zone_selection.is_selecting {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(RichText::new("Drawing zone on overlay... Press ESC to cancel").color(Color32::YELLOW));
            });
            ui.add_space(4.0);
        }

        // Show error if any
        if let Some(ref error) = view_state.zone_selection_error {
            ui.horizontal(|ui| {
                ui.label(RichText::new("⚠").color(Color32::RED));
                ui.label(RichText::new(error).color(Color32::RED));
            });
            if ui.small_button("Dismiss").clicked() {
                view_state.zone_selection_error = None;
            }
            ui.add_space(4.0);
        }

        // Show zone overlays toggle
        ui.checkbox(&mut view_state.show_zone_overlays, "Show zones on preview");

        ui.add_space(8.0);

        // Zone list with scrolling
        let available_height = max_height - 100.0;
        egui::ScrollArea::vertical()
            .max_height(available_height.max(100.0))
            .show(ui, |ui| {
                if view_state.ocr_zones.is_empty() {
                    ui.label(RichText::new("No zones defined").italics().color(Color32::GRAY));
                    ui.add_space(8.0);
                    ui.label("Click 'Create Zone' to draw a region on the overlay.");
                } else {
                    // Render each zone
                    let mut zone_to_delete: Option<usize> = None;
                    let mut zone_to_toggle: Option<usize> = None;
                    let mut zone_to_configure: Option<usize> = None;

                    for (idx, zone) in view_state.ocr_zones.iter().enumerate() {
                        let ocr_result = view_state.zone_ocr_results.get(&zone.id);

                        ui.push_id(idx, |ui| {
                            render_zone_item(
                                ui,
                                zone,
                                ocr_result,
                                &mut zone_to_toggle,
                                &mut zone_to_delete,
                                &mut zone_to_configure,
                                idx,
                            );
                        });

                        ui.add_space(4.0);
                    }

                    // Handle toggle
                    if let Some(idx) = zone_to_toggle {
                        if let Some(zone) = view_state.ocr_zones.get_mut(idx) {
                            zone.enabled = !zone.enabled;
                            view_state.zones_dirty = true;
                        }
                    }

                    // Handle delete
                    if let Some(idx) = zone_to_delete {
                        let zone_id = view_state.ocr_zones[idx].id.clone();
                        view_state.ocr_zones.remove(idx);
                        view_state.zone_ocr_results.remove(&zone_id);
                        view_state.zones_dirty = true;
                    }

                    // Handle settings
                    if let Some(idx) = zone_to_configure {
                        if let Some(zone) = view_state.ocr_zones.get(idx) {
                            // Initialize pending settings from zone's current settings
                            view_state.zone_selection.settings_zone_index = Some(idx);
                            view_state.zone_selection.show_settings_dialog = true;
                            view_state.zone_selection.pending_use_custom_preprocessing = zone.preprocessing.is_some();
                            view_state.zone_selection.pending_preprocessing = zone.preprocessing
                                .clone()
                                .unwrap_or_else(|| view_state.preprocessing.clone());
                        }
                    }
                }
            });

        // Zone naming dialog
        if view_state.zone_selection.show_naming_dialog {
            render_zone_naming_dialog(ui, view_state);
        }

        // Zone settings dialog
        if view_state.zone_selection.show_settings_dialog {
            render_zone_settings_dialog(ui, view_state);
        }
    });
}

/// Render a single zone item in the list
fn render_zone_item(
    ui: &mut egui::Ui,
    zone: &OcrRegion,
    ocr_result: Option<&ZoneOcrResult>,
    zone_to_toggle: &mut Option<usize>,
    zone_to_delete: &mut Option<usize>,
    zone_to_configure: &mut Option<usize>,
    idx: usize,
) {
    let is_enabled = zone.enabled;
    let has_custom_preprocessing = zone.preprocessing.is_some();
    let bg_color = if is_enabled {
        Color32::from_rgba_unmultiplied(40, 40, 50, 255)
    } else {
        Color32::from_rgba_unmultiplied(30, 30, 35, 255)
    };

    egui::Frame::none()
        .fill(bg_color)
        .rounding(Rounding::same(4.0))
        .stroke(Stroke::new(1.0, Color32::from_gray(60)))
        .inner_margin(8.0)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                // Zone name and enable toggle
                let name_color = if is_enabled {
                    Color32::WHITE
                } else {
                    Color32::GRAY
                };

                ui.label(RichText::new(&zone.name).color(name_color).strong());

                // Show indicator if custom preprocessing is enabled
                if has_custom_preprocessing {
                    ui.label(RichText::new("*").color(Color32::from_rgb(255, 200, 100)).small());
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Delete button
                    if ui.small_button("X").clicked() {
                        *zone_to_delete = Some(idx);
                    }

                    // Settings button
                    let settings_color = if has_custom_preprocessing {
                        Color32::from_rgb(255, 200, 100)
                    } else {
                        Color32::GRAY
                    };
                    if ui.button(RichText::new("...").color(settings_color)).on_hover_text("Zone OCR settings").clicked() {
                        *zone_to_configure = Some(idx);
                    }

                    // Enable/disable toggle
                    let toggle_text = if is_enabled { "On" } else { "Off" };
                    let toggle_color = if is_enabled {
                        Color32::from_rgb(0, 200, 0)
                    } else {
                        Color32::GRAY
                    };

                    if ui.button(RichText::new(toggle_text).color(toggle_color)).clicked() {
                        *zone_to_toggle = Some(idx);
                    }
                });
            });

            // OCR value
            if let Some(result) = ocr_result {
                ui.horizontal(|ui| {
                    ui.label("Value:");
                    ui.label(
                        RichText::new(&result.text)
                            .color(Color32::from_rgb(100, 200, 255))
                            .monospace(),
                    );
                });

                // Time since last update
                let elapsed = result.last_updated.elapsed();
                let elapsed_text = if elapsed.as_secs() < 1 {
                    format!("{:.1}s ago", elapsed.as_secs_f32())
                } else {
                    format!("{}s ago", elapsed.as_secs())
                };
                ui.label(RichText::new(elapsed_text).small().color(Color32::GRAY));
            } else if is_enabled {
                ui.label(RichText::new("No value").italics().color(Color32::GRAY));
            }

            // Show bounds info
            let bounds_text = format!(
                "Region: {:.1}%, {:.1}% - {:.1}% x {:.1}%",
                zone.bounds.0 * 100.0,
                zone.bounds.1 * 100.0,
                zone.bounds.2 * 100.0,
                zone.bounds.3 * 100.0
            );
            ui.label(RichText::new(bounds_text).small().color(Color32::from_gray(100)));
        });
}

/// Render the zone naming dialog
fn render_zone_naming_dialog(ui: &mut egui::Ui, view_state: &mut VisionViewState) {
    egui::Window::new("Name Zone")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
        .show(ui.ctx(), |ui| {
            ui.vertical(|ui| {
                ui.label("Enter a name for this zone:");
                ui.add_space(4.0);

                ui.text_edit_singleline(&mut view_state.zone_selection.pending_zone_name);

                ui.add_space(8.0);

                ui.label("Content type:");
                egui::ComboBox::from_id_salt("content_type")
                    .selected_text(content_type_name(&view_state.zone_selection.pending_content_type))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut view_state.zone_selection.pending_content_type,
                            ContentType::Text,
                            "Text",
                        );
                        ui.selectable_value(
                            &mut view_state.zone_selection.pending_content_type,
                            ContentType::Number,
                            "Number",
                        );
                        ui.selectable_value(
                            &mut view_state.zone_selection.pending_content_type,
                            ContentType::Percentage,
                            "Percentage",
                        );
                        ui.selectable_value(
                            &mut view_state.zone_selection.pending_content_type,
                            ContentType::Time,
                            "Time",
                        );
                    });

                ui.add_space(12.0);

                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        view_state.zone_selection.show_naming_dialog = false;
                        view_state.zone_selection.current_selection = None;
                        view_state.zone_selection.pending_zone_name.clear();
                    }

                    let can_save = !view_state.zone_selection.pending_zone_name.trim().is_empty()
                        && view_state.zone_selection.current_selection.is_some();

                    if ui.add_enabled(can_save, egui::Button::new("Save")).clicked() {
                        if let Some(bounds) = view_state.zone_selection.current_selection {
                            let new_zone = OcrRegion {
                                id: Uuid::new_v4().to_string(),
                                name: view_state.zone_selection.pending_zone_name.trim().to_string(),
                                bounds,
                                content_type: view_state.zone_selection.pending_content_type.clone(),
                                enabled: true,
                                preprocessing: None, // Use global settings by default
                            };

                            view_state.ocr_zones.push(new_zone);
                            view_state.zones_dirty = true;

                            // Reset dialog state
                            view_state.zone_selection.show_naming_dialog = false;
                            view_state.zone_selection.current_selection = None;
                            view_state.zone_selection.pending_zone_name.clear();
                            view_state.zone_selection.pending_content_type = ContentType::Text;
                        }
                    }
                });
            });
        });
}

fn content_type_name(content_type: &ContentType) -> &'static str {
    match content_type {
        ContentType::Text => "Text",
        ContentType::Number => "Number",
        ContentType::Percentage => "Percentage",
        ContentType::Time => "Time",
    }
}

/// Render the zone settings dialog
fn render_zone_settings_dialog(ui: &mut egui::Ui, view_state: &mut VisionViewState) {
    let zone_name = view_state.zone_selection.settings_zone_index
        .and_then(|idx| view_state.ocr_zones.get(idx))
        .map(|z| z.name.clone())
        .unwrap_or_else(|| "Zone".to_string());

    egui::Window::new(format!("Settings: {}", zone_name))
        .collapsible(false)
        .resizable(false)
        .min_width(320.0)
        .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
        .show(ui.ctx(), |ui| {
            ui.vertical(|ui| {
                // Custom preprocessing toggle
                ui.checkbox(
                    &mut view_state.zone_selection.pending_use_custom_preprocessing,
                    "Use custom preprocessing for this zone",
                );

                ui.add_space(8.0);

                // Preprocessing settings (disabled if not using custom)
                let use_custom = view_state.zone_selection.pending_use_custom_preprocessing;

                ui.add_enabled_ui(use_custom, |ui| {
                    egui::Frame::none()
                        .fill(Color32::from_rgba_unmultiplied(30, 30, 40, 255))
                        .rounding(Rounding::same(4.0))
                        .inner_margin(12.0)
                        .show(ui, |ui| {
                            let pp = &mut view_state.zone_selection.pending_preprocessing;

                            ui.checkbox(&mut pp.enabled, "Enable preprocessing");

                            ui.add_space(4.0);

                            ui.add_enabled_ui(pp.enabled, |ui| {
                                // Grayscale
                                ui.checkbox(&mut pp.grayscale, "Grayscale");

                                // Invert colors
                                ui.checkbox(&mut pp.invert, "Invert colors");

                                ui.add_space(4.0);

                                // Contrast
                                ui.horizontal(|ui| {
                                    ui.label("Contrast:");
                                    ui.add(egui::Slider::new(&mut pp.contrast, 0.5..=3.0).step_by(0.1));
                                });

                                // Sharpen
                                ui.horizontal(|ui| {
                                    ui.label("Sharpen:");
                                    ui.add(egui::Slider::new(&mut pp.sharpen, 0.0..=2.0).step_by(0.1));
                                });

                                // Scale
                                ui.horizontal(|ui| {
                                    ui.label("Scale:");
                                    let mut scale_val = pp.scale as i32;
                                    if ui.add(egui::Slider::new(&mut scale_val, 1..=4)).changed() {
                                        pp.scale = scale_val as u32;
                                    }
                                    ui.label("x");
                                });
                            });
                        });
                });

                if !use_custom {
                    ui.add_space(4.0);
                    ui.label(RichText::new("Using global preprocessing settings").italics().color(Color32::GRAY));
                }

                ui.add_space(12.0);

                // Buttons
                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        view_state.zone_selection.show_settings_dialog = false;
                        view_state.zone_selection.settings_zone_index = None;
                    }

                    if ui.button("Save").clicked() {
                        if let Some(idx) = view_state.zone_selection.settings_zone_index {
                            if let Some(zone) = view_state.ocr_zones.get_mut(idx) {
                                // Apply the settings
                                if view_state.zone_selection.pending_use_custom_preprocessing {
                                    zone.preprocessing = Some(view_state.zone_selection.pending_preprocessing.clone());
                                } else {
                                    zone.preprocessing = None;
                                }
                                view_state.zones_dirty = true;
                            }
                        }

                        view_state.zone_selection.show_settings_dialog = false;
                        view_state.zone_selection.settings_zone_index = None;
                    }
                });
            });
        });
}

/// Draw zone overlays on the preview image
pub fn draw_zone_overlays(
    ui: &egui::Ui,
    zones: &[OcrRegion],
    image_rect: egui::Rect,
    zone_results: &std::collections::HashMap<String, ZoneOcrResult>,
) {
    let painter = ui.painter();

    for zone in zones {
        // Convert normalized bounds to image coordinates
        let zone_rect = egui::Rect::from_min_size(
            egui::pos2(
                image_rect.min.x + zone.bounds.0 * image_rect.width(),
                image_rect.min.y + zone.bounds.1 * image_rect.height(),
            ),
            egui::vec2(
                zone.bounds.2 * image_rect.width(),
                zone.bounds.3 * image_rect.height(),
            ),
        );

        // Choose color based on enabled state
        let (fill_color, stroke_color, text_color) = if zone.enabled {
            (
                Color32::from_rgba_unmultiplied(0, 200, 0, 30),
                Color32::from_rgb(0, 200, 0),
                Color32::from_rgb(0, 255, 0),
            )
        } else {
            (
                Color32::from_rgba_unmultiplied(128, 128, 128, 20),
                Color32::from_gray(128),
                Color32::from_gray(180),
            )
        };

        // Draw zone rectangle
        painter.rect_filled(zone_rect, Rounding::same(2.0), fill_color);
        painter.rect_stroke(zone_rect, Rounding::same(2.0), Stroke::new(1.5, stroke_color));

        // Draw zone name
        let label_pos = zone_rect.left_top() + egui::vec2(2.0, 2.0);
        painter.text(
            label_pos,
            egui::Align2::LEFT_TOP,
            &zone.name,
            egui::FontId::proportional(11.0),
            text_color,
        );

        // Draw current value if available
        if let Some(result) = zone_results.get(&zone.id) {
            let value_pos = zone_rect.left_top() + egui::vec2(2.0, 14.0);
            painter.text(
                value_pos,
                egui::Align2::LEFT_TOP,
                &result.text,
                egui::FontId::monospace(10.0),
                Color32::from_rgb(100, 200, 255),
            );
        }
    }
}
