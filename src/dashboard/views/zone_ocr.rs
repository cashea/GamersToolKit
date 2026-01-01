//! Zone OCR management view
//!
//! Dashboard panel for creating, editing, and managing OCR zones.

use egui::{Color32, RichText, Rounding, Stroke, Vec2};
use uuid::Uuid;

use crate::dashboard::components::add_scroll_slider;
use crate::dashboard::state::{AutoConfigureState, AutoConfigureStep, VisionViewState, ZoneOcrResult};
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
                let status_text = if view_state.zone_selection.repositioning_zone_index.is_some() {
                    "Repositioning zone on overlay... Press ESC to cancel"
                } else {
                    "Drawing zone on overlay... Press ESC to cancel"
                };
                ui.label(RichText::new(status_text).color(Color32::YELLOW));
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
            .id_salt("zone_ocr_list")
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
                    let mut zone_to_reposition: Option<usize> = None;
                    let is_selecting = view_state.zone_selection.is_selecting;

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
                                &mut zone_to_reposition,
                                idx,
                                is_selecting,
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
                            view_state.zone_selection.pending_content_type = zone.content_type.clone();
                            view_state.zone_selection.pending_use_custom_preprocessing = zone.preprocessing.is_some();
                            view_state.zone_selection.pending_preprocessing = zone.preprocessing
                                .clone()
                                .unwrap_or_else(|| view_state.preprocessing.clone());
                        }
                    }

                    // Handle reposition
                    if let Some(idx) = zone_to_reposition {
                        view_state.zone_selection.repositioning_zone_index = Some(idx);
                        view_state.pending_zone_selection_mode = true;
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
    zone_to_reposition: &mut Option<usize>,
    idx: usize,
    is_selecting: bool,
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

                    // Reposition button (disabled while selecting)
                    if ui.add_enabled(!is_selecting, egui::Button::new("[]").small())
                        .on_hover_text("Reposition zone")
                        .clicked()
                    {
                        *zone_to_reposition = Some(idx);
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
    let Some(idx) = view_state.zone_selection.settings_zone_index else {
        return;
    };

    let zone_name = view_state.ocr_zones.get(idx)
        .map(|z| z.name.clone())
        .unwrap_or_else(|| "Zone".to_string());

    let mut should_close = false;

    egui::Window::new(format!("Settings: {}", zone_name))
        .collapsible(false)
        .resizable(false)
        .min_width(320.0)
        .show(ui.ctx(), |ui| {
            ui.vertical(|ui| {
                // Get current zone settings
                let Some(zone) = view_state.ocr_zones.get_mut(idx) else {
                    should_close = true;
                    return;
                };

                // Content type selector - apply immediately
                ui.horizontal(|ui| {
                    ui.label("Content type:");
                    let mut content_type = zone.content_type.clone();
                    let response = egui::ComboBox::from_id_salt("settings_content_type")
                        .selected_text(content_type_name(&content_type))
                        .show_ui(ui, |ui| {
                            let mut changed = false;
                            if ui.selectable_value(&mut content_type, ContentType::Text, "Text (any characters)").changed() {
                                changed = true;
                            }
                            if ui.selectable_value(&mut content_type, ContentType::Number, "Number (digits only)").changed() {
                                changed = true;
                            }
                            if ui.selectable_value(&mut content_type, ContentType::Percentage, "Percentage (digits + %)").changed() {
                                changed = true;
                            }
                            if ui.selectable_value(&mut content_type, ContentType::Time, "Time (digits + :)").changed() {
                                changed = true;
                            }
                            changed
                        });
                    if response.inner.unwrap_or(false) {
                        zone.content_type = content_type;
                        view_state.zones_dirty = true;
                    }
                });

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(8.0);

                // Custom preprocessing toggle - apply immediately
                let mut use_custom = zone.preprocessing.is_some();
                if ui.checkbox(&mut use_custom, "Use custom preprocessing for this zone").changed() {
                    if use_custom {
                        // Enable custom: copy global settings as starting point
                        zone.preprocessing = Some(view_state.preprocessing.clone());
                    } else {
                        // Disable custom: use global settings
                        zone.preprocessing = None;
                    }
                    view_state.zones_dirty = true;
                }

                ui.add_space(8.0);

                // Preprocessing settings (disabled if not using custom)
                ui.add_enabled_ui(use_custom, |ui| {
                    egui::Frame::none()
                        .fill(Color32::from_rgba_unmultiplied(30, 30, 40, 255))
                        .rounding(Rounding::same(4.0))
                        .inner_margin(12.0)
                        .show(ui, |ui| {
                            // Need to re-get zone as mutable for preprocessing changes
                            let Some(zone) = view_state.ocr_zones.get_mut(idx) else {
                                return;
                            };
                            let Some(pp) = zone.preprocessing.as_mut() else {
                                return;
                            };

                            if ui.checkbox(&mut pp.enabled, "Enable preprocessing").changed() {
                                view_state.zones_dirty = true;
                            }

                            ui.add_space(4.0);

                            ui.add_enabled_ui(pp.enabled, |ui| {
                                // Grayscale
                                if ui.checkbox(&mut pp.grayscale, "Grayscale").changed() {
                                    view_state.zones_dirty = true;
                                }

                                // Invert colors
                                if ui.checkbox(&mut pp.invert, "Invert colors").changed() {
                                    view_state.zones_dirty = true;
                                }

                                ui.add_space(4.0);

                                // Contrast
                                ui.horizontal(|ui| {
                                    ui.label("Contrast:");
                                    if add_scroll_slider(ui, &mut pp.contrast, 0.5..=3.0, Some(0.1), None, Some(1)).changed() {
                                        view_state.zones_dirty = true;
                                    }
                                });

                                // Sharpen
                                ui.horizontal(|ui| {
                                    ui.label("Sharpen:");
                                    if add_scroll_slider(ui, &mut pp.sharpen, 0.0..=2.0, Some(0.1), None, Some(1)).changed() {
                                        view_state.zones_dirty = true;
                                    }
                                });

                                // Scale
                                ui.horizontal(|ui| {
                                    ui.label("Scale:");
                                    let mut scale_val = pp.scale as i32;
                                    if add_scroll_slider(ui, &mut scale_val, 1..=4, Some(1.0), Some("x"), None).changed() {
                                        pp.scale = scale_val as u32;
                                        view_state.zones_dirty = true;
                                    }
                                });
                            });
                        });
                });

                if !use_custom {
                    ui.add_space(4.0);
                    ui.label(RichText::new("Using global preprocessing settings").italics().color(Color32::GRAY));
                }

                ui.add_space(12.0);
                ui.separator();
                ui.add_space(8.0);

                // Auto Configure section
                ui.label(RichText::new("Auto Configure").strong());
                ui.add_space(4.0);
                ui.label(RichText::new("Automatically find settings that produce OCR text.").small().color(Color32::GRAY));
                ui.add_space(8.0);

                // Check if auto-configure is running for this zone
                let is_auto_configuring = view_state.zone_selection.auto_configure
                    .as_ref()
                    .map(|ac| ac.zone_index == idx && ac.current_step != AutoConfigureStep::Completed)
                    .unwrap_or(false);

                if is_auto_configuring {
                    // Show progress
                    if let Some(ref ac) = view_state.zone_selection.auto_configure {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label(&ac.status_message);
                        });

                        // Progress bar
                        let progress = ac.current_combination as f32 / ac.total_combinations.max(1) as f32;
                        ui.add(egui::ProgressBar::new(progress).show_percentage());

                        ui.add_space(4.0);

                        // Cancel button
                        if ui.button("Cancel").clicked() {
                            view_state.zone_selection.auto_configure = None;
                        }
                    }
                } else {
                    // Show result if just completed
                    if let Some(ref ac) = view_state.zone_selection.auto_configure {
                        if ac.zone_index == idx && ac.current_step == AutoConfigureStep::Completed {
                            if ac.success {
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new("✓").color(Color32::GREEN));
                                    ui.label(RichText::new(format!("Best config found! (conf: {:.0}%)", ac.best_confidence * 100.0)).color(Color32::GREEN));
                                });
                                if !ac.best_text.is_empty() {
                                    ui.label(RichText::new(format!("Text: '{}'", ac.best_text)).small().color(Color32::LIGHT_GRAY));
                                }
                            } else if let Some(ref err) = ac.error_message {
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new("✗").color(Color32::RED));
                                    ui.label(RichText::new(err).color(Color32::RED));
                                });
                            }
                            ui.add_space(4.0);
                        }
                    }

                    // Start button
                    if ui.button("Auto Configure").on_hover_text("Tests all settings combinations and picks the best one").clicked() {
                        // Calculate total combinations: 4 scales * 2 preprocessing * 2 grayscale * 2 invert * 3 contrast = 96
                        let total = 4 * 2 * 2 * 2 * 3;
                        view_state.zone_selection.auto_configure = Some(AutoConfigureState {
                            zone_index: idx,
                            current_step: AutoConfigureStep::Starting,
                            current_scale: 1,
                            current_preprocessing_enabled: false,
                            current_grayscale: false,
                            current_invert: false,
                            current_contrast: 1.0,
                            total_combinations: total,
                            current_combination: 0,
                            status_message: "Starting...".to_string(),
                            success: false,
                            error_message: None,
                            best_preprocessing: None,
                            best_confidence: 0.0,
                            best_text: String::new(),
                        });
                    }
                }

                ui.add_space(12.0);
                ui.separator();
                ui.add_space(8.0);

                // Zone actions
                ui.horizontal(|ui| {
                    if ui.button("Reposition Zone").on_hover_text("Draw a new boundary for this zone").clicked() {
                        // Set up repositioning mode
                        view_state.zone_selection.repositioning_zone_index = Some(idx);
                        view_state.pending_zone_selection_mode = true;
                        should_close = true;
                    }
                });

                ui.add_space(12.0);

                // Close button
                ui.horizontal(|ui| {
                    if ui.button("Close").clicked() {
                        should_close = true;
                    }
                });
            });
        });

    if should_close {
        view_state.zone_selection.show_settings_dialog = false;
        view_state.zone_selection.settings_zone_index = None;
    }
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
