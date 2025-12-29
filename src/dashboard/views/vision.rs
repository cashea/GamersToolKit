//! Vision view - OCR testing and preview

use egui::RichText;
use parking_lot::{Mutex, RwLock};
use std::sync::Arc;

use crate::capture::ScreenCapture;
use crate::dashboard::state::VisionViewState;
use crate::dashboard::theme::ThemeColors;
use crate::shared::SharedAppState;
use crate::vision::OcrBackend;

/// Render the vision/OCR view
pub fn render_vision_view(
    ui: &mut egui::Ui,
    view_state: &mut VisionViewState,
    shared_state: &Arc<RwLock<SharedAppState>>,
    capture_manager: &Arc<Mutex<Option<ScreenCapture>>>,
) {
    ui.heading(RichText::new("Vision / OCR").size(24.0).strong());
    ui.add_space(8.0);
    ui.label(
        RichText::new("Test OCR text detection on captured frames")
            .size(14.0)
            .color(ThemeColors::TEXT_SECONDARY)
    );

    ui.add_space(24.0);

    // Model status section
    render_model_status(ui, view_state);

    ui.add_space(16.0);
    ui.separator();
    ui.add_space(16.0);

    // Two-column layout: Preview and Results
    let available_width = ui.available_width();
    let use_two_columns = available_width > 700.0;

    // Get a frame for OCR if capturing
    let preview_frame = {
        let capture_guard = capture_manager.lock();
        if let Some(ref capture) = *capture_guard {
            capture.try_next_frame()
        } else {
            None
        }
    };

    if use_two_columns {
        ui.columns(2, |columns| {
            render_preview_column(&mut columns[0], view_state, shared_state, preview_frame);
            render_results_column(&mut columns[1], view_state);
        });
    } else {
        render_preview_column(ui, view_state, shared_state, preview_frame);
        ui.add_space(16.0);
        render_results_column(ui, view_state);
    }
}

/// Render OCR backend selection and status
fn render_model_status(ui: &mut egui::Ui, view_state: &mut VisionViewState) {
    egui::Frame::none()
        .fill(ThemeColors::BG_MEDIUM)
        .rounding(egui::Rounding::same(8.0))
        .inner_margin(12.0)
        .show(ui, |ui| {
            // Backend selection
            ui.horizontal(|ui| {
                ui.heading(RichText::new("OCR Backend").size(16.0));
                ui.add_space(16.0);

                // Backend selector
                egui::ComboBox::from_id_salt("ocr_backend")
                    .selected_text(match view_state.selected_backend {
                        OcrBackend::WindowsOcr => "Windows OCR (Recommended)",
                        OcrBackend::PaddleOcr => "PaddleOCR (ONNX)",
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
            });

            ui.add_space(12.0);

            // Backend-specific status
            match view_state.selected_backend {
                OcrBackend::WindowsOcr => {
                    render_windows_ocr_status(ui, view_state);
                }
                OcrBackend::PaddleOcr => {
                    render_paddle_ocr_status(ui, view_state);
                }
            }

            // Show errors if any
            if let Some(ref error) = view_state.last_error {
                ui.add_space(8.0);
                ui.label(RichText::new(format!("Error: {}", error)).color(ThemeColors::ACCENT_ERROR));
            }
        });
}

/// Render Windows OCR status
fn render_windows_ocr_status(ui: &mut egui::Ui, view_state: &mut VisionViewState) {
    ui.horizontal(|ui| {
        ui.label("Status:");
        if view_state.windows_ocr_initialized {
            ui.label(RichText::new("Ready").color(ThemeColors::ACCENT_SUCCESS));
        } else {
            ui.label(RichText::new("Not initialized").color(ThemeColors::TEXT_MUTED));
        }
    });

    ui.add_space(4.0);
    ui.label(
        RichText::new("Uses built-in Windows text recognition. Fast and accurate for game UI.")
            .size(11.0)
            .color(ThemeColors::TEXT_MUTED)
    );

    ui.add_space(8.0);

    let init_enabled = !view_state.windows_ocr_initialized && !view_state.is_processing;
    ui.add_enabled_ui(init_enabled, |ui| {
        if ui.button("Initialize Windows OCR").clicked() {
            view_state.pending_init = true;
        }
    });
}

/// Render PaddleOCR status
fn render_paddle_ocr_status(ui: &mut egui::Ui, view_state: &mut VisionViewState) {
    ui.horizontal(|ui| {
        ui.label("Status:");
        if view_state.is_downloading {
            ui.spinner();
            ui.label(RichText::new("Downloading models...").color(ThemeColors::ACCENT_WARNING));
        } else if view_state.ocr_initialized {
            ui.label(RichText::new("Ready").color(ThemeColors::ACCENT_SUCCESS));
        } else if view_state.models_ready {
            ui.label(RichText::new("Models loaded, not initialized").color(ThemeColors::ACCENT_WARNING));
        } else {
            ui.label(RichText::new("Models not downloaded").color(ThemeColors::TEXT_MUTED));
        }
    });

    ui.add_space(4.0);

    // Model list
    ui.horizontal(|ui| {
        ui.label("Detection:");
        let det_status = if view_state.detection_model_ready {
            RichText::new("OK").color(ThemeColors::ACCENT_SUCCESS)
        } else {
            RichText::new("--").color(ThemeColors::TEXT_MUTED)
        };
        ui.label(det_status);

        ui.add_space(16.0);

        ui.label("Recognition:");
        let rec_status = if view_state.recognition_model_ready {
            RichText::new("OK").color(ThemeColors::ACCENT_SUCCESS)
        } else {
            RichText::new("--").color(ThemeColors::TEXT_MUTED)
        };
        ui.label(rec_status);
    });

    ui.add_space(4.0);
    ui.label(
        RichText::new("Uses PaddleOCR with ONNX Runtime. Requires model download (~15MB).")
            .size(11.0)
            .color(ThemeColors::TEXT_MUTED)
    );

    ui.add_space(8.0);

    // Download/Initialize buttons
    ui.horizontal(|ui| {
        let download_enabled = !view_state.is_downloading && !view_state.models_ready;
        ui.add_enabled_ui(download_enabled, |ui| {
            if ui.button("Download Models").clicked() {
                view_state.pending_download = true;
            }
        });

        let init_enabled = !view_state.ocr_initialized && view_state.models_ready;
        ui.add_enabled_ui(init_enabled, |ui| {
            if ui.button("Initialize OCR").clicked() {
                view_state.pending_init = true;
            }
        });
    });

    // Show download progress if active
    if view_state.is_downloading {
        ui.add_space(8.0);
        let progress = view_state.download_progress;
        ui.add(egui::ProgressBar::new(progress).text(format!("{:.0}%", progress * 100.0)));
    }
}

/// Render the preview column
fn render_preview_column(
    ui: &mut egui::Ui,
    view_state: &mut VisionViewState,
    shared_state: &Arc<RwLock<SharedAppState>>,
    preview_frame: Option<crate::capture::frame::CapturedFrame>,
) {
    egui::Frame::none()
        .fill(ThemeColors::BG_MEDIUM)
        .rounding(egui::Rounding::same(8.0))
        .inner_margin(12.0)
        .show(ui, |ui| {
            ui.heading(RichText::new("Capture Preview").size(16.0));
            ui.add_space(8.0);

            // Controls
            ui.horizontal(|ui| {
                ui.checkbox(&mut view_state.auto_run_ocr, "Auto-run OCR");
                ui.add_space(16.0);

                // Check if selected backend is initialized
                let backend_ready = match view_state.selected_backend {
                    OcrBackend::WindowsOcr => view_state.windows_ocr_initialized,
                    OcrBackend::PaddleOcr => view_state.ocr_initialized,
                };
                let run_enabled = backend_ready && !view_state.is_processing;
                ui.add_enabled_ui(run_enabled, |ui| {
                    if ui.button("Run OCR Now").clicked() {
                        view_state.pending_ocr_run = true;
                    }
                });

                if view_state.is_processing {
                    ui.spinner();
                }
            });

            ui.add_space(12.0);

            // Preview area
            let preview_size = egui::vec2(400.0, 225.0);
            egui::Frame::none()
                .fill(ThemeColors::BG_DARK)
                .rounding(egui::Rounding::same(6.0))
                .show(ui, |ui| {
                    ui.set_min_size(preview_size);

                    if let Some(frame) = preview_frame {
                        // Store frame for OCR processing
                        view_state.last_frame_data = Some(frame.data.clone());
                        view_state.last_frame_width = frame.width;
                        view_state.last_frame_height = frame.height;

                        // Update texture
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

                        // Render preview with OCR overlays
                        if let Some(ref texture) = view_state.preview_texture {
                            let tex_size = texture.size_vec2();
                            let scale = (preview_size.x / tex_size.x).min(preview_size.y / tex_size.y);
                            let scaled_size = tex_size * scale;

                            // Calculate offset for centering
                            let offset_x = (preview_size.x - scaled_size.x) / 2.0;
                            let offset_y = (preview_size.y - scaled_size.y) / 2.0;

                            let (rect, _response) = ui.allocate_exact_size(preview_size, egui::Sense::hover());

                            // Draw the image
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

                            // Draw OCR bounding boxes if available
                            if view_state.show_bounding_boxes {
                                let scale_x = scaled_size.x / frame.width as f32;
                                let scale_y = scaled_size.y / frame.height as f32;

                                for detection in &view_state.last_ocr_results {
                                    let (x, y, w, h) = detection.bounds;
                                    let box_rect = egui::Rect::from_min_size(
                                        image_rect.min + egui::vec2(x as f32 * scale_x, y as f32 * scale_y),
                                        egui::vec2(w as f32 * scale_x, h as f32 * scale_y),
                                    );

                                    // Draw box
                                    ui.painter().rect_stroke(
                                        box_rect,
                                        egui::Rounding::ZERO,
                                        egui::Stroke::new(2.0, ThemeColors::ACCENT_PRIMARY),
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
                                "No capture active\nStart capture in Capture tab"
                            };
                            ui.label(
                                RichText::new(message)
                                    .size(14.0)
                                    .color(ThemeColors::TEXT_MUTED)
                            );
                        });
                    }
                });

            // Display options
            ui.add_space(8.0);
            ui.checkbox(&mut view_state.show_bounding_boxes, "Show bounding boxes");
        });
}

/// Render the results column
fn render_results_column(ui: &mut egui::Ui, view_state: &mut VisionViewState) {
    egui::Frame::none()
        .fill(ThemeColors::BG_MEDIUM)
        .rounding(egui::Rounding::same(8.0))
        .inner_margin(12.0)
        .show(ui, |ui| {
            ui.heading(RichText::new("OCR Results").size(16.0));
            ui.add_space(8.0);

            // Stats
            ui.horizontal(|ui| {
                ui.label(format!("Detected regions: {}", view_state.last_ocr_results.len()));
                ui.add_space(16.0);
                if view_state.last_processing_time_ms > 0 {
                    ui.label(
                        RichText::new(format!("Processing time: {}ms", view_state.last_processing_time_ms))
                            .color(ThemeColors::TEXT_MUTED)
                    );
                }
            });

            ui.add_space(12.0);

            // Confidence threshold slider
            ui.horizontal(|ui| {
                ui.label("Min confidence:");
                ui.add(egui::Slider::new(&mut view_state.confidence_threshold, 0.0..=1.0)
                    .step_by(0.05)
                    .fixed_decimals(2));
            });

            ui.add_space(12.0);
            ui.separator();
            ui.add_space(8.0);

            // Results list
            egui::ScrollArea::vertical()
                .max_height(300.0)
                .show(ui, |ui| {
                    let filtered_results: Vec<_> = view_state.last_ocr_results
                        .iter()
                        .filter(|r| r.confidence >= view_state.confidence_threshold)
                        .collect();

                    if filtered_results.is_empty() {
                        ui.label(
                            RichText::new("No text detected")
                                .color(ThemeColors::TEXT_MUTED)
                        );
                    } else {
                        for (idx, result) in filtered_results.iter().enumerate() {
                            egui::Frame::none()
                                .fill(ThemeColors::BG_DARK)
                                .rounding(egui::Rounding::same(4.0))
                                .inner_margin(8.0)
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            RichText::new(format!("#{}", idx + 1))
                                                .color(ThemeColors::ACCENT_PRIMARY)
                                                .strong()
                                        );
                                        ui.label(
                                            RichText::new(format!("{:.0}%", result.confidence * 100.0))
                                                .color(confidence_color(result.confidence))
                                        );
                                    });

                                    ui.label(&result.text);

                                    let (x, y, w, h) = result.bounds;
                                    ui.label(
                                        RichText::new(format!("Pos: ({}, {}) Size: {}x{}", x, y, w, h))
                                            .size(10.0)
                                            .color(ThemeColors::TEXT_MUTED)
                                    );
                                });

                            ui.add_space(4.0);
                        }
                    }
                });
        });
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
