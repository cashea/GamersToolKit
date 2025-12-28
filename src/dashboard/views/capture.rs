//! Capture view - Screen capture target selection and preview

use egui::RichText;
use parking_lot::{Mutex, RwLock};
use std::sync::Arc;
use std::time::Instant;

use crate::capture::{CaptureTarget, ScreenCapture};
use crate::dashboard::state::CaptureViewState;
use crate::dashboard::theme::ThemeColors;
use crate::shared::{CaptureCommand, SharedAppState};

/// Render the capture view
pub fn render_capture_view(
    ui: &mut egui::Ui,
    view_state: &mut CaptureViewState,
    shared_state: &Arc<RwLock<SharedAppState>>,
    capture_manager: &Arc<Mutex<Option<ScreenCapture>>>,
) {
    ui.heading(RichText::new("Screen Capture").size(24.0).strong());
    ui.add_space(8.0);
    ui.label(
        RichText::new("Select a window or monitor to capture")
            .size(14.0)
            .color(ThemeColors::TEXT_SECONDARY)
    );

    ui.add_space(24.0);

    // Refresh button and status
    ui.horizontal(|ui| {
        if ui.button("Refresh Sources").clicked() {
            refresh_sources(view_state);
        }

        ui.add_space(16.0);

        if let Some(last_refresh) = view_state.last_refresh {
            let elapsed = last_refresh.elapsed().as_secs();
            ui.label(
                RichText::new(format!("Last refreshed: {}s ago", elapsed))
                    .size(12.0)
                    .color(ThemeColors::TEXT_MUTED)
            );
        }
    });

    // Auto-refresh on first load
    if view_state.last_refresh.is_none() {
        refresh_sources(view_state);
    }

    ui.add_space(16.0);

    // Tab selection
    ui.horizontal(|ui| {
        let window_selected = view_state.target_type == 0;
        let monitor_selected = view_state.target_type == 1;

        if ui.selectable_label(window_selected, "Windows").clicked() {
            view_state.target_type = 0;
        }
        ui.add_space(8.0);
        if ui.selectable_label(monitor_selected, "Monitors").clicked() {
            view_state.target_type = 1;
        }
    });

    ui.add_space(16.0);

    // Search/filter
    ui.horizontal(|ui| {
        ui.label("Filter:");
        ui.add_space(8.0);
        ui.add(
            egui::TextEdit::singleline(&mut view_state.search_query)
                .hint_text("Type to filter...")
                .desired_width(200.0)
        );
        if !view_state.search_query.is_empty() {
            if ui.small_button("Clear").clicked() {
                view_state.search_query.clear();
            }
        }
    });

    ui.add_space(16.0);

    // Content area with list and preview side by side
    let available_width = ui.available_width();
    let use_two_columns = available_width > 700.0;

    // Get a frame for preview if capturing and preview is enabled
    let preview_frame = if view_state.preview_enabled {
        let capture_guard = capture_manager.lock();
        if let Some(ref capture) = *capture_guard {
            capture.try_next_frame()
        } else {
            None
        }
    } else {
        None
    };

    if use_two_columns {
        ui.columns(2, |columns| {
            render_source_list_column(&mut columns[0], view_state);
            render_capture_settings_column(&mut columns[1], view_state, shared_state, preview_frame);
        });
    } else {
        render_source_list_column(ui, view_state);
        ui.add_space(16.0);
        render_capture_settings_column(ui, view_state, shared_state, preview_frame);
    }
}

/// Render the source list column
fn render_source_list_column(ui: &mut egui::Ui, view_state: &mut CaptureViewState) {
    egui::Frame::none()
        .fill(ThemeColors::BG_MEDIUM)
        .rounding(egui::Rounding::same(8.0))
        .inner_margin(12.0)
        .show(ui, |ui| {
            ui.set_max_height(400.0);

            egui::ScrollArea::vertical().show(ui, |ui| {
                if view_state.target_type == 0 {
                    render_window_list(ui, view_state);
                } else {
                    render_monitor_list(ui, view_state);
                }
            });
        });
}

/// Render the capture settings column
fn render_capture_settings_column(
    ui: &mut egui::Ui,
    view_state: &mut CaptureViewState,
    shared_state: &Arc<RwLock<SharedAppState>>,
    preview_frame: Option<crate::capture::frame::CapturedFrame>,
) {
    egui::Frame::none()
        .fill(ThemeColors::BG_MEDIUM)
        .rounding(egui::Rounding::same(8.0))
        .inner_margin(16.0)
        .show(ui, |ui| {
            ui.heading(RichText::new("Capture Settings").size(16.0));
            ui.add_space(12.0);

            // Current selection
            let selection_text = get_current_selection_text(view_state);
            ui.horizontal(|ui| {
                ui.label(RichText::new("Selected:").color(ThemeColors::TEXT_MUTED));
                ui.label(RichText::new(selection_text).strong());
            });

            ui.add_space(12.0);

            // Preview toggle
            ui.checkbox(&mut view_state.preview_enabled, "Enable preview");

            // Preview area (compact)
            if view_state.preview_enabled {
                ui.add_space(8.0);
                egui::Frame::none()
                    .fill(ThemeColors::BG_DARK)
                    .rounding(egui::Rounding::same(6.0))
                    .show(ui, |ui| {
                        let preview_size = egui::vec2(320.0, 180.0);
                        ui.set_min_size(preview_size);

                        // Update texture if we have a new frame
                        if let Some(frame) = preview_frame {
                            let needs_update = view_state.preview_frame_size
                                .map(|(w, h)| w != frame.width || h != frame.height)
                                .unwrap_or(true)
                                || view_state.preview_texture.is_none();

                            let color_image = egui::ColorImage::from_rgba_unmultiplied(
                                [frame.width as usize, frame.height as usize],
                                &frame.data,
                            );

                            if needs_update {
                                // Create new texture
                                let texture = ui.ctx().load_texture(
                                    "capture_preview",
                                    color_image,
                                    egui::TextureOptions::LINEAR,
                                );
                                view_state.preview_texture = Some(texture);
                                view_state.preview_frame_size = Some((frame.width, frame.height));
                            } else if let Some(ref mut texture) = view_state.preview_texture {
                                // Update existing texture
                                texture.set(color_image, egui::TextureOptions::LINEAR);
                            }
                        }

                        // Render the preview texture or placeholder
                        if let Some(ref texture) = view_state.preview_texture {
                            let tex_size = texture.size_vec2();
                            // Scale to fit preview area while maintaining aspect ratio
                            let scale = (preview_size.x / tex_size.x).min(preview_size.y / tex_size.y);
                            let scaled_size = tex_size * scale;

                            ui.centered_and_justified(|ui| {
                                ui.image((texture.id(), scaled_size));
                            });
                        } else {
                            ui.centered_and_justified(|ui| {
                                let is_capturing = shared_state.read().runtime.is_capturing;
                                let message = if is_capturing {
                                    "Loading preview..."
                                } else {
                                    "Preview not available\n(Capture not running)"
                                };
                                ui.label(
                                    RichText::new(message)
                                        .size(11.0)
                                        .color(ThemeColors::TEXT_MUTED)
                                );
                            });
                        }
                    });
            } else {
                // Clear texture when preview is disabled to free memory
                view_state.preview_texture = None;
                view_state.preview_frame_size = None;
            }

            ui.add_space(16.0);
            ui.separator();
            ui.add_space(12.0);

            // Start/Stop capture button (moved up for visibility)
            let is_capturing = shared_state.read().runtime.is_capturing;
            let capture_btn_text = if is_capturing { "Stop Capture" } else { "Start Capture" };
            let capture_btn_color = if is_capturing {
                ThemeColors::ACCENT_ERROR
            } else {
                ThemeColors::ACCENT_SUCCESS
            };

            if ui.add(
                egui::Button::new(RichText::new(capture_btn_text).color(egui::Color32::WHITE))
                    .fill(capture_btn_color)
                    .min_size(egui::vec2(140.0, 32.0))
            ).clicked() {
                let mut state = shared_state.write();
                state.runtime.capture_command = Some(if is_capturing {
                    CaptureCommand::Stop
                } else {
                    CaptureCommand::Start
                });
            }

            // Show current FPS if capturing
            if is_capturing {
                ui.add_space(4.0);
                let fps = shared_state.read().runtime.capture_fps;
                ui.label(
                    RichText::new(format!("Capturing at {:.1} FPS", fps))
                        .color(ThemeColors::ACCENT_SUCCESS)
                );
            }

            ui.add_space(8.0);

            // Apply button
            let has_selection = view_state.selected_window.is_some() || view_state.selected_monitor.is_some();
            ui.add_enabled_ui(has_selection, |ui| {
                if ui.add(
                    egui::Button::new(
                        RichText::new("Apply Selection")
                            .color(if has_selection { egui::Color32::WHITE } else { ThemeColors::TEXT_MUTED })
                    )
                    .fill(if has_selection { ThemeColors::ACCENT_PRIMARY } else { ThemeColors::BG_LIGHT })
                    .min_size(egui::vec2(140.0, 32.0))
                ).clicked() {
                    apply_selection(view_state, shared_state);
                }
            });
        });
}

/// Refresh available capture sources
fn refresh_sources(view_state: &mut CaptureViewState) {
    // Get available windows
    view_state.available_windows = ScreenCapture::list_windows()
        .unwrap_or_default();

    // Get available monitors
    view_state.available_monitors = ScreenCapture::list_monitors()
        .unwrap_or_default();

    view_state.last_refresh = Some(Instant::now());
}

/// Render the window list
fn render_window_list(ui: &mut egui::Ui, view_state: &mut CaptureViewState) {
    let filter = view_state.search_query.to_lowercase();
    let filtered_windows: Vec<_> = view_state.available_windows
        .iter()
        .enumerate()
        .filter(|(_, w)| filter.is_empty() || w.to_lowercase().contains(&filter))
        .collect();

    if filtered_windows.is_empty() {
        ui.label(
            RichText::new("No windows found")
                .color(ThemeColors::TEXT_MUTED)
        );
        return;
    }

    for (idx, window) in filtered_windows {
        let is_selected = view_state.selected_window == Some(idx);
        let response = ui.selectable_label(is_selected, window);

        if response.clicked() {
            view_state.selected_window = Some(idx);
            view_state.selected_monitor = None;
        }
    }
}

/// Render the monitor list
fn render_monitor_list(ui: &mut egui::Ui, view_state: &mut CaptureViewState) {
    let filter = view_state.search_query.to_lowercase();
    let filtered_monitors: Vec<_> = view_state.available_monitors
        .iter()
        .enumerate()
        .filter(|(_, m)| filter.is_empty() || m.to_lowercase().contains(&filter))
        .collect();

    if filtered_monitors.is_empty() {
        ui.label(
            RichText::new("No monitors found")
                .color(ThemeColors::TEXT_MUTED)
        );
        return;
    }

    for (idx, monitor) in filtered_monitors {
        let is_selected = view_state.selected_monitor == Some(idx);
        let response = ui.selectable_label(is_selected, monitor);

        if response.clicked() {
            view_state.selected_monitor = Some(idx);
            view_state.selected_window = None;
        }
    }
}

/// Get text describing the current selection
fn get_current_selection_text(view_state: &CaptureViewState) -> String {
    if let Some(idx) = view_state.selected_window {
        view_state.available_windows
            .get(idx)
            .cloned()
            .unwrap_or_else(|| "Invalid selection".to_string())
    } else if let Some(idx) = view_state.selected_monitor {
        view_state.available_monitors
            .get(idx)
            .cloned()
            .unwrap_or_else(|| "Invalid selection".to_string())
    } else {
        "None".to_string()
    }
}

/// Apply the current selection to shared state
fn apply_selection(view_state: &CaptureViewState, shared_state: &Arc<RwLock<SharedAppState>>) {
    let mut state = shared_state.write();

    if let Some(idx) = view_state.selected_window {
        if let Some(window_title) = view_state.available_windows.get(idx) {
            state.capture_config.target = CaptureTarget::Window(window_title.clone());
            state.config.capture.target_window = Some(window_title.clone());
            state.runtime.current_capture_target = Some(window_title.clone());
        }
    } else if let Some(idx) = view_state.selected_monitor {
        if idx == 0 {
            state.capture_config.target = CaptureTarget::PrimaryMonitor;
            state.runtime.current_capture_target = Some("Primary Monitor".to_string());
        } else {
            state.capture_config.target = CaptureTarget::MonitorIndex(idx);
            state.runtime.current_capture_target = view_state.available_monitors.get(idx).cloned();
        }
        state.config.capture.target_window = None;
    }
}
