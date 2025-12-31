//! Vision view - OCR testing and preview

use egui::RichText;
use parking_lot::{Mutex, RwLock};
use std::sync::Arc;

use crate::capture::ScreenCapture;
use crate::dashboard::state::{OcrGranularity, VisionViewState};
use crate::dashboard::theme::ThemeColors;
use crate::dashboard::views::zone_ocr::{render_zone_ocr_panel, draw_zone_overlays};
use crate::shared::SharedAppState;
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

    // Two-column layout for wide screens, single column for narrow
    if available_width > 800.0 {
        // Two columns: Preview | Zone OCR
        ui.columns(2, |columns| {
            render_preview_panel(&mut columns[0], view_state, shared_state, preview_frame, available_height);
            render_zone_ocr_panel(&mut columns[1], view_state, available_height);
        });
    } else {
        // Single column
        render_preview_panel(ui, view_state, shared_state, preview_frame, available_height * 0.5);
        ui.add_space(8.0);
        render_zone_ocr_panel(ui, view_state, available_height * 0.4);
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

            // Header
            ui.label(RichText::new("Preview").size(17.0).strong());

            ui.add_space(6.0);

            // Preview area - maintain aspect ratio of captured frame
            let available = ui.available_size();
            let preview_width = available.x - 4.0;

            // Calculate height based on frame aspect ratio, or use 16:9 default
            let aspect_ratio = if view_state.last_frame_width > 0 && view_state.last_frame_height > 0 {
                view_state.last_frame_width as f32 / view_state.last_frame_height as f32
            } else {
                16.0 / 9.0 // Default to 16:9
            };

            // Height based on aspect ratio, but cap at reasonable max
            let max_preview_height = (available.y - 150.0).max(100.0); // Reserve space for controls
            let aspect_height = preview_width / aspect_ratio;
            let preview_height = aspect_height.min(max_preview_height);
            let preview_size = egui::vec2(preview_width, preview_height);

            egui::Frame::none()
                .fill(ThemeColors::BG_DARK)
                .rounding(egui::Rounding::same(4.0))
                .show(ui, |ui| {
                    ui.set_min_size(preview_size);
                    ui.set_max_size(preview_size);

                    // Update texture if a new frame arrived
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
                    }

                    // Display the texture if we have one (persists between frames)
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

                        // Draw bounding boxes using stored frame dimensions
                        if view_state.show_bounding_boxes && view_state.last_frame_width > 0 {
                            let scale_x = scaled_size.x / view_state.last_frame_width as f32;
                            let scale_y = scaled_size.y / view_state.last_frame_height as f32;
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

                        // Draw zone overlays if enabled
                        if view_state.show_zone_overlays && !view_state.ocr_zones.is_empty() {
                            draw_zone_overlays(ui, &view_state.ocr_zones, image_rect, &view_state.zone_ocr_results);
                        }
                    } else {
                        // No texture yet - show placeholder
                        ui.centered_and_justified(|ui| {
                            let is_capturing = shared_state.read().runtime.is_capturing;
                            let message = if is_capturing {
                                "Waiting for first frame..."
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

            // Preprocessing controls (collapsible)
            ui.add_space(4.0);
            egui::CollapsingHeader::new(RichText::new("Preprocessing").size(14.0))
                .default_open(view_state.preprocessing.enabled)
                .show(ui, |ui| {
                    // Track if any setting changed to trigger OCR re-run
                    let mut settings_changed = false;

                    ui.horizontal(|ui| {
                        let enabled_checkbox = ui.checkbox(&mut view_state.preprocessing.enabled, "Enable");
                        if enabled_checkbox.changed() {
                            settings_changed = true;
                        }
                        enabled_checkbox.on_hover_text("Apply image filters before OCR to improve accuracy");
                    });

                    ui.add_enabled_ui(view_state.preprocessing.enabled, |ui| {
                        ui.horizontal(|ui| {
                            if ui.checkbox(&mut view_state.preprocessing.grayscale, "Grayscale").changed() {
                                settings_changed = true;
                            }
                            if ui.checkbox(&mut view_state.preprocessing.invert, "Invert").changed() {
                                settings_changed = true;
                            }
                        });

                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Contrast:").size(13.0));
                            if ui.add(egui::Slider::new(&mut view_state.preprocessing.contrast, 0.5..=3.0)
                                .step_by(0.1)
                                .fixed_decimals(1)).changed() {
                                settings_changed = true;
                            }
                        });

                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Sharpen:").size(13.0));
                            if ui.add(egui::Slider::new(&mut view_state.preprocessing.sharpen, 0.0..=1.0)
                                .step_by(0.1)
                                .fixed_decimals(1)).changed() {
                                settings_changed = true;
                            }
                        });

                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Scale:").size(13.0));
                            let scale_slider = ui.add(egui::Slider::new(&mut view_state.preprocessing.scale, 1..=4)
                                .step_by(1.0)
                                .suffix("x"));
                            if scale_slider.changed() {
                                settings_changed = true;
                            }
                            scale_slider.on_hover_text("Upscale image before OCR (2-3x recommended for small text)");
                        });
                    });

                    // Auto-trigger OCR when preprocessing settings change
                    if settings_changed && !view_state.is_processing {
                        let backend_ready = match view_state.selected_backend {
                            OcrBackend::WindowsOcr => view_state.windows_ocr_initialized,
                            OcrBackend::PaddleOcr => view_state.ocr_initialized,
                        };
                        if backend_ready && view_state.last_frame_data.is_some() {
                            view_state.pending_ocr_run = true;
                        }
                    }
                });
        });
}

