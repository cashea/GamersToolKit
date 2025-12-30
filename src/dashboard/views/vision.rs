//! Vision view - OCR testing and preview

use egui::RichText;
use parking_lot::{Mutex, RwLock};
use std::sync::Arc;

use crate::capture::ScreenCapture;
use crate::dashboard::state::{OcrGranularity, VisionViewState};
use crate::dashboard::theme::ThemeColors;
use crate::shared::SharedAppState;
use crate::storage::profiles::LabeledRegion;
use crate::vision::OcrBackend;

/// Render the vision/OCR view
pub fn render_vision_view(
    ui: &mut egui::Ui,
    view_state: &mut VisionViewState,
    shared_state: &Arc<RwLock<SharedAppState>>,
    capture_manager: &Arc<Mutex<Option<ScreenCapture>>>,
) {
    // Get a frame for OCR if capturing
    let preview_frame = {
        let capture_guard = capture_manager.lock();
        if let Some(ref capture) = *capture_guard {
            capture.try_next_frame()
        } else {
            None
        }
    };

    // Header with OCR backend controls (compact)
    ui.horizontal(|ui| {
        ui.heading(RichText::new("Vision / OCR").size(24.0).strong());
        ui.add_space(24.0);
        render_ocr_backend_inline(ui, view_state);
    });
    ui.add_space(12.0);

    // Main content - use available height
    let available_height = ui.available_height();
    let available_width = ui.available_width();

    // Three-column layout for wide screens, two columns for medium
    if available_width > 1000.0 {
        // Three columns: Preview | OCR Results | Labeling
        ui.columns(3, |columns| {
            render_preview_panel(&mut columns[0], view_state, shared_state, preview_frame, available_height);
            render_results_panel(&mut columns[1], view_state, available_height);
            render_labeling_panel(&mut columns[2], view_state, available_height);
        });
    } else if available_width > 600.0 {
        // Two columns: Preview | Results+Labeling stacked
        ui.columns(2, |columns| {
            render_preview_panel(&mut columns[0], view_state, shared_state, preview_frame, available_height);
            // Stack results and labeling in the right column
            let half_height = (available_height - 16.0) / 2.0;
            render_results_panel(&mut columns[1], view_state, half_height);
            columns[1].add_space(8.0);
            render_labeling_panel(&mut columns[1], view_state, half_height);
        });
    } else {
        // Single column with tabs or compact view
        render_preview_panel(ui, view_state, shared_state, preview_frame, available_height * 0.4);
        ui.add_space(8.0);
        render_results_panel(ui, view_state, available_height * 0.3);
        ui.add_space(8.0);
        render_labeling_panel(ui, view_state, available_height * 0.25);
    }
}

/// Render inline OCR backend selector and status
fn render_ocr_backend_inline(ui: &mut egui::Ui, view_state: &mut VisionViewState) {
    // Backend selector
    egui::ComboBox::from_id_salt("ocr_backend")
        .selected_text(match view_state.selected_backend {
            OcrBackend::WindowsOcr => "Windows OCR",
            OcrBackend::PaddleOcr => "PaddleOCR",
        })
        .show_ui(ui, |ui| {
            ui.selectable_value(
                &mut view_state.selected_backend,
                OcrBackend::WindowsOcr,
                "Windows OCR (Recommended)",
            );
            ui.selectable_value(
                &mut view_state.selected_backend,
                OcrBackend::PaddleOcr,
                "PaddleOCR (ONNX)",
            );
        });

    ui.add_space(8.0);

    // Granularity selector (Word vs Line)
    egui::ComboBox::from_id_salt("ocr_granularity")
        .selected_text(match view_state.ocr_granularity {
            OcrGranularity::Word => "Words",
            OcrGranularity::Line => "Lines",
        })
        .width(70.0)
        .show_ui(ui, |ui| {
            ui.selectable_value(
                &mut view_state.ocr_granularity,
                OcrGranularity::Word,
                "Words",
            );
            ui.selectable_value(
                &mut view_state.ocr_granularity,
                OcrGranularity::Line,
                "Lines",
            );
        });

    ui.add_space(8.0);

    // Status indicator
    let (status_text, status_color, needs_init) = match view_state.selected_backend {
        OcrBackend::WindowsOcr => {
            if view_state.windows_ocr_initialized {
                ("Ready", ThemeColors::ACCENT_SUCCESS, false)
            } else {
                ("Not initialized", ThemeColors::TEXT_MUTED, true)
            }
        }
        OcrBackend::PaddleOcr => {
            if view_state.ocr_initialized {
                ("Ready", ThemeColors::ACCENT_SUCCESS, false)
            } else if view_state.models_ready {
                ("Models ready", ThemeColors::ACCENT_WARNING, true)
            } else {
                ("Need models", ThemeColors::TEXT_MUTED, false)
            }
        }
    };

    ui.label(RichText::new(status_text).color(status_color));

    // Init button if needed
    if needs_init && !view_state.is_processing {
        if ui.small_button("Initialize").clicked() {
            view_state.pending_init = true;
        }
    }

    // Download button for PaddleOCR
    if view_state.selected_backend == OcrBackend::PaddleOcr
        && !view_state.models_ready
        && !view_state.is_downloading
    {
        if ui.small_button("Download Models").clicked() {
            view_state.pending_download = true;
        }
    }

    if view_state.is_downloading {
        ui.spinner();
    }

    // Error display
    if let Some(ref error) = view_state.last_error {
        ui.label(RichText::new(format!("Error: {}", error)).color(ThemeColors::ACCENT_ERROR).size(14.0));
    }
}

/// Render the preview panel with height constraint
fn render_preview_panel(
    ui: &mut egui::Ui,
    view_state: &mut VisionViewState,
    shared_state: &Arc<RwLock<SharedAppState>>,
    preview_frame: Option<crate::capture::frame::CapturedFrame>,
    max_height: f32,
) {
    egui::Frame::none()
        .fill(ThemeColors::BG_MEDIUM)
        .rounding(egui::Rounding::same(8.0))
        .inner_margin(10.0)
        .show(ui, |ui| {
            ui.set_max_height(max_height);

            // Header with controls
            ui.horizontal(|ui| {
                ui.label(RichText::new("Preview").size(17.0).strong());
                ui.add_space(8.0);
                ui.checkbox(&mut view_state.auto_run_ocr, "Auto");

                let backend_ready = match view_state.selected_backend {
                    OcrBackend::WindowsOcr => view_state.windows_ocr_initialized,
                    OcrBackend::PaddleOcr => view_state.ocr_initialized,
                };
                let run_enabled = backend_ready && !view_state.is_processing;
                ui.add_enabled_ui(run_enabled, |ui| {
                    if ui.small_button("Run OCR").clicked() {
                        view_state.pending_ocr_run = true;
                    }
                });

                if view_state.is_processing {
                    ui.spinner();
                }
            });

            ui.add_space(6.0);

            // Preview area - use remaining height
            let available = ui.available_size();
            let preview_height = (available.y - 30.0).max(100.0);
            let preview_size = egui::vec2(available.x - 4.0, preview_height);

            egui::Frame::none()
                .fill(ThemeColors::BG_DARK)
                .rounding(egui::Rounding::same(4.0))
                .show(ui, |ui| {
                    ui.set_min_size(preview_size);

                    if let Some(frame) = preview_frame {
                        view_state.last_frame_data = Some(frame.data.clone());
                        view_state.last_frame_width = frame.width;
                        view_state.last_frame_height = frame.height;

                        let color_image = egui::ColorImage::from_rgba_unmultiplied(
                            [frame.width as usize, frame.height as usize],
                            &frame.data,
                        );

                        let needs_update = view_state.preview_frame_size
                            .map(|(w, h)| w != frame.width || h != frame.height)
                            .unwrap_or(true)
                            || view_state.preview_texture.is_none();

                        if needs_update {
                            let texture = ui.ctx().load_texture(
                                "vision_preview",
                                color_image,
                                egui::TextureOptions::LINEAR,
                            );
                            view_state.preview_texture = Some(texture);
                            view_state.preview_frame_size = Some((frame.width, frame.height));
                        } else if let Some(ref mut texture) = view_state.preview_texture {
                            texture.set(color_image, egui::TextureOptions::LINEAR);
                        }

                        if let Some(ref texture) = view_state.preview_texture {
                            let tex_size = texture.size_vec2();
                            let scale = (preview_size.x / tex_size.x).min(preview_size.y / tex_size.y);
                            let scaled_size = tex_size * scale;
                            let offset_x = (preview_size.x - scaled_size.x) / 2.0;
                            let offset_y = (preview_size.y - scaled_size.y) / 2.0;

                            let (rect, _) = ui.allocate_exact_size(preview_size, egui::Sense::hover());
                            let image_rect = egui::Rect::from_min_size(
                                rect.min + egui::vec2(offset_x, offset_y),
                                scaled_size,
                            );
                            ui.painter().image(
                                texture.id(),
                                image_rect,
                                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                                egui::Color32::WHITE,
                            );

                            if view_state.show_bounding_boxes {
                                let scale_x = scaled_size.x / frame.width as f32;
                                let scale_y = scaled_size.y / frame.height as f32;
                                for detection in &view_state.last_ocr_results {
                                    let (x, y, w, h) = detection.bounds;
                                    let box_rect = egui::Rect::from_min_size(
                                        image_rect.min + egui::vec2(x as f32 * scale_x, y as f32 * scale_y),
                                        egui::vec2(w as f32 * scale_x, h as f32 * scale_y),
                                    );
                                    ui.painter().rect_stroke(
                                        box_rect,
                                        egui::Rounding::ZERO,
                                        egui::Stroke::new(1.5, ThemeColors::ACCENT_PRIMARY),
                                    );
                                }
                            }
                        }
                    } else {
                        ui.centered_and_justified(|ui| {
                            let is_capturing = shared_state.read().runtime.is_capturing;
                            let message = if is_capturing {
                                "Waiting for frame..."
                            } else {
                                "Start capture first"
                            };
                            ui.label(RichText::new(message).size(15.0).color(ThemeColors::TEXT_MUTED));
                        });
                    }
                });

            // Options row
            ui.horizontal(|ui| {
                ui.checkbox(&mut view_state.show_bounding_boxes, "Boxes");
                if view_state.last_processing_time_ms > 0 {
                    ui.label(RichText::new(format!("{}ms", view_state.last_processing_time_ms)).size(14.0).color(ThemeColors::TEXT_MUTED));
                }
            });
        });
}

/// Render the results panel with height constraint
fn render_results_panel(ui: &mut egui::Ui, view_state: &mut VisionViewState, max_height: f32) {
    egui::Frame::none()
        .fill(ThemeColors::BG_MEDIUM)
        .rounding(egui::Rounding::same(8.0))
        .inner_margin(10.0)
        .show(ui, |ui| {
            ui.set_max_height(max_height);

            // Header with count
            ui.horizontal(|ui| {
                ui.label(RichText::new("OCR Results").size(17.0).strong());
                ui.label(RichText::new(format!("({})", view_state.last_ocr_results.len())).size(15.0).color(ThemeColors::TEXT_MUTED));
            });

            ui.add_space(4.0);

            // Confidence slider (compact)
            ui.horizontal(|ui| {
                ui.label(RichText::new("Min:").size(14.0));
                ui.add(egui::Slider::new(&mut view_state.confidence_threshold, 0.0..=1.0)
                    .step_by(0.05)
                    .fixed_decimals(2)
                    .show_value(true));
            });

            ui.add_space(4.0);

            // Results list with scroll
            let scroll_height = (max_height - 70.0).max(50.0);
            egui::ScrollArea::vertical()
                .max_height(scroll_height)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    let filtered_results: Vec<_> = view_state.last_ocr_results
                        .iter()
                        .filter(|r| r.confidence >= view_state.confidence_threshold)
                        .collect();

                    if filtered_results.is_empty() {
                        ui.label(RichText::new("No text detected").size(14.0).color(ThemeColors::TEXT_MUTED));
                    } else {
                        for (idx, result) in filtered_results.iter().enumerate() {
                            egui::Frame::none()
                                .fill(ThemeColors::BG_DARK)
                                .rounding(egui::Rounding::same(3.0))
                                .inner_margin(6.0)
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(RichText::new(format!("#{}", idx + 1)).size(14.0).color(ThemeColors::ACCENT_PRIMARY).strong());
                                        ui.label(RichText::new(format!("{:.0}%", result.confidence * 100.0)).size(13.0).color(confidence_color(result.confidence)));
                                    });
                                    ui.label(RichText::new(&result.text).size(14.0));
                                });
                            ui.add_space(2.0);
                        }
                    }
                });
        });
}

/// Render the labeling panel with height constraint
fn render_labeling_panel(ui: &mut egui::Ui, view_state: &mut VisionViewState, max_height: f32) {
    egui::Frame::none()
        .fill(ThemeColors::BG_MEDIUM)
        .rounding(egui::Rounding::same(8.0))
        .inner_margin(10.0)
        .show(ui, |ui| {
            ui.set_max_height(max_height);

            // Header with save button
            ui.horizontal(|ui| {
                ui.label(RichText::new("Labeling").size(17.0).strong());
                ui.label(RichText::new(format!("({})", view_state.labeled_regions.len())).size(15.0).color(ThemeColors::TEXT_MUTED));

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Show dirty indicator and save button
                    if view_state.labels_dirty {
                        if ui.small_button("Save").clicked() {
                            view_state.pending_profile_save = true;
                        }
                        ui.label(RichText::new("*").size(14.0).color(ThemeColors::ACCENT_WARNING));
                    }
                });
            });

            ui.add_space(4.0);

            // Search input
            ui.horizontal(|ui| {
                ui.label(RichText::new("Find:").size(14.0));
                let search_response = ui.add(
                    egui::TextEdit::singleline(&mut view_state.region_search_text)
                        .hint_text("Search...")
                        .desired_width(100.0)
                );
                if search_response.changed() {
                    update_matching_regions(view_state);
                }
                if ui.small_button("X").clicked() {
                    view_state.region_search_text.clear();
                    view_state.matching_regions.clear();
                    view_state.selected_region_index = None;
                }
            });

            ui.add_space(4.0);

            // Calculate remaining height for the two sections
            let remaining_height = (max_height - 80.0).max(100.0);
            let section_height = remaining_height / 2.0;

            // Matching regions section
            ui.label(RichText::new("Matches:").size(14.0).color(ThemeColors::TEXT_MUTED));
            egui::ScrollArea::vertical()
                .id_salt("matching_regions")
                .max_height(section_height)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    if view_state.region_search_text.is_empty() {
                        ui.label(RichText::new("Enter search text").size(13.0).color(ThemeColors::TEXT_MUTED));
                    } else if view_state.matching_regions.is_empty() {
                        ui.label(RichText::new("No matches").size(13.0).color(ThemeColors::ACCENT_WARNING));
                    } else {
                        let matching_indices: Vec<usize> = view_state.matching_regions.clone();
                        for &idx in &matching_indices {
                            if let Some(result) = view_state.last_ocr_results.get(idx) {
                                let is_selected = view_state.selected_region_index == Some(idx);
                                let bg = if is_selected { ThemeColors::ACCENT_PRIMARY.gamma_multiply(0.3) } else { ThemeColors::BG_DARK };

                                let response = egui::Frame::none()
                                    .fill(bg)
                                    .rounding(egui::Rounding::same(3.0))
                                    .inner_margin(4.0)
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            ui.label(RichText::new(format!("#{}", idx + 1)).size(13.0).color(ThemeColors::ACCENT_PRIMARY));
                                            ui.label(RichText::new(&result.text).size(13.0));
                                        });
                                    }).response;

                                if response.interact(egui::Sense::click()).clicked() {
                                    view_state.selected_region_index = Some(idx);
                                    view_state.pending_label.clear();
                                }
                                ui.add_space(2.0);
                            }
                        }
                    }
                });

            ui.add_space(4.0);

            // Label input (when region selected)
            if let Some(selected_idx) = view_state.selected_region_index {
                if let Some(result) = view_state.last_ocr_results.get(selected_idx).cloned() {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Label:").size(14.0));
                        ui.add(egui::TextEdit::singleline(&mut view_state.pending_label).hint_text("Name...").desired_width(80.0));
                        if !view_state.pending_label.trim().is_empty() {
                            if ui.small_button("Save").clicked() {
                                let labeled = LabeledRegion {
                                    label: view_state.pending_label.trim().to_string(),
                                    matched_text: result.text.clone(),
                                    bounds: result.bounds,
                                    confidence: result.confidence,
                                };
                                view_state.labeled_regions.push(labeled);
                                view_state.pending_label.clear();
                                view_state.selected_region_index = None;
                                view_state.labels_dirty = true;
                            }
                        }
                    });
                }
            }

            ui.add_space(4.0);

            // Labeled regions list
            ui.label(RichText::new("Labels:").size(14.0).color(ThemeColors::TEXT_MUTED));
            egui::ScrollArea::vertical()
                .id_salt("labeled_regions")
                .max_height(section_height)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    if view_state.labeled_regions.is_empty() {
                        ui.label(RichText::new("No labels yet").size(13.0).color(ThemeColors::TEXT_MUTED));
                    } else {
                        let mut to_delete: Option<usize> = None;
                        for (idx, labeled) in view_state.labeled_regions.iter().enumerate() {
                            // Get live value if available
                            let live_value = view_state.labeled_regions_live
                                .get(idx)
                                .and_then(|live| live.current_text.as_ref());

                            egui::Frame::none()
                                .fill(ThemeColors::BG_DARK)
                                .rounding(egui::Rounding::same(3.0))
                                .inner_margin(4.0)
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(RichText::new(&labeled.label).size(14.0).color(ThemeColors::ACCENT_SUCCESS).strong());
                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                            if ui.small_button("X").clicked() {
                                                to_delete = Some(idx);
                                            }
                                        });
                                    });
                                    // Show current live value (or original if no live value)
                                    if let Some(current) = live_value {
                                        ui.label(RichText::new(current).size(13.0));
                                    } else {
                                        // No live match - show original with indicator
                                        ui.label(RichText::new(&labeled.matched_text).size(13.0).color(ThemeColors::TEXT_MUTED).italics());
                                    }
                                });
                            ui.add_space(2.0);
                        }
                        if let Some(idx) = to_delete {
                            view_state.labeled_regions.remove(idx);
                            // Also remove from live values
                            if idx < view_state.labeled_regions_live.len() {
                                view_state.labeled_regions_live.remove(idx);
                            }
                            view_state.labels_dirty = true;
                        }
                    }
                });
        });
}

/// Update the matching regions based on search text
fn update_matching_regions(view_state: &mut VisionViewState) {
    view_state.matching_regions.clear();
    view_state.selected_region_index = None;

    let search = view_state.region_search_text.to_lowercase();
    if search.is_empty() {
        return;
    }

    for (idx, result) in view_state.last_ocr_results.iter().enumerate() {
        if result.text.to_lowercase().contains(&search) {
            view_state.matching_regions.push(idx);
        }
    }
}

/// Get color based on confidence level
fn confidence_color(confidence: f32) -> egui::Color32 {
    if confidence >= 0.9 {
        ThemeColors::ACCENT_SUCCESS
    } else if confidence >= 0.7 {
        ThemeColors::ACCENT_WARNING
    } else {
        ThemeColors::ACCENT_ERROR
    }
}
