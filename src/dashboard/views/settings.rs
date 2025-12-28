//! Settings view - Application configuration

use egui::RichText;
use parking_lot::RwLock;
use std::sync::Arc;
use std::cell::Cell;

use crate::dashboard::state::{SettingsSection, SettingsViewState};
use crate::dashboard::theme::ThemeColors;
use crate::shared::SharedAppState;

/// Render the settings view
pub fn render_settings_view(
    ui: &mut egui::Ui,
    view_state: &mut SettingsViewState,
    shared_state: &Arc<RwLock<SharedAppState>>,
) {
    ui.heading(RichText::new("Settings").size(24.0).strong());
    ui.add_space(8.0);
    ui.label(
        RichText::new("Configure application behavior and preferences")
            .size(14.0)
            .color(ThemeColors::TEXT_SECONDARY)
    );

    ui.add_space(24.0);

    // Track changes using Cell to avoid borrow issues
    let changed = Cell::new(false);

    egui::ScrollArea::vertical().show(ui, |ui| {
        // General Settings
        let is_general_expanded = view_state.expanded_section == Some(SettingsSection::General);
        egui::Frame::none()
            .fill(ThemeColors::BG_MEDIUM)
            .rounding(egui::Rounding::same(8.0))
            .inner_margin(16.0)
            .show(ui, |ui| {
                let header_response = ui.horizontal(|ui| {
                    let arrow = if is_general_expanded { "v" } else { ">" };
                    ui.label(RichText::new(arrow).size(12.0).color(ThemeColors::TEXT_MUTED));
                    ui.add_space(8.0);
                    ui.heading(RichText::new("General").size(16.0));
                }).response;

                if header_response.interact(egui::Sense::click()).clicked() {
                    view_state.expanded_section = if is_general_expanded { None } else { Some(SettingsSection::General) };
                }

                if is_general_expanded {
                    ui.add_space(16.0);
                    ui.separator();
                    ui.add_space(12.0);

                    let mut state = shared_state.write();

                    ui.horizontal(|ui| {
                        ui.label("Start minimized:");
                        ui.add_space(8.0);
                        if ui.checkbox(&mut state.config.general.start_minimized, "").changed() {
                            changed.set(true);
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("Auto-start with Windows:");
                        ui.add_space(8.0);
                        if ui.checkbox(&mut state.config.general.auto_start, "").changed() {
                            changed.set(true);
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("Check for updates:");
                        ui.add_space(8.0);
                        if ui.checkbox(&mut state.config.general.check_updates, "").changed() {
                            changed.set(true);
                        }
                    });
                }
            });

        ui.add_space(16.0);

        // Capture Settings
        let is_capture_expanded = view_state.expanded_section == Some(SettingsSection::Capture);
        egui::Frame::none()
            .fill(ThemeColors::BG_MEDIUM)
            .rounding(egui::Rounding::same(8.0))
            .inner_margin(16.0)
            .show(ui, |ui| {
                let header_response = ui.horizontal(|ui| {
                    let arrow = if is_capture_expanded { "v" } else { ">" };
                    ui.label(RichText::new(arrow).size(12.0).color(ThemeColors::TEXT_MUTED));
                    ui.add_space(8.0);
                    ui.heading(RichText::new("Capture").size(16.0));
                }).response;

                if header_response.interact(egui::Sense::click()).clicked() {
                    view_state.expanded_section = if is_capture_expanded { None } else { Some(SettingsSection::Capture) };
                }

                if is_capture_expanded {
                    ui.add_space(16.0);
                    ui.separator();
                    ui.add_space(12.0);

                    let mut state = shared_state.write();

                    ui.horizontal(|ui| {
                        ui.label("Max FPS:");
                        ui.add_space(8.0);
                        let mut fps = state.config.capture.max_fps as f32;
                        if ui.add(egui::Slider::new(&mut fps, 1.0..=60.0).suffix(" fps")).changed() {
                            state.config.capture.max_fps = fps as u32;
                            changed.set(true);
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("Capture cursor:");
                        ui.add_space(8.0);
                        if ui.checkbox(&mut state.config.capture.capture_cursor, "").changed() {
                            changed.set(true);
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("Draw capture border:");
                        ui.add_space(8.0);
                        if ui.checkbox(&mut state.config.capture.draw_border, "").changed() {
                            changed.set(true);
                        }
                    });

                    let mut target = state.config.capture.target_window.clone().unwrap_or_default();
                    drop(state); // Release lock before text edit
                    ui.horizontal(|ui| {
                        ui.label("Target window:");
                        ui.add_space(8.0);
                        if ui.text_edit_singleline(&mut target).changed() {
                            let mut state = shared_state.write();
                            state.config.capture.target_window = if target.is_empty() { None } else { Some(target) };
                            changed.set(true);
                        }
                    });
                }
            });

        ui.add_space(16.0);

        // Overlay Settings
        let is_overlay_expanded = view_state.expanded_section == Some(SettingsSection::Overlay);
        egui::Frame::none()
            .fill(ThemeColors::BG_MEDIUM)
            .rounding(egui::Rounding::same(8.0))
            .inner_margin(16.0)
            .show(ui, |ui| {
                let header_response = ui.horizontal(|ui| {
                    let arrow = if is_overlay_expanded { "v" } else { ">" };
                    ui.label(RichText::new(arrow).size(12.0).color(ThemeColors::TEXT_MUTED));
                    ui.add_space(8.0);
                    ui.heading(RichText::new("Overlay").size(16.0));
                }).response;

                if header_response.interact(egui::Sense::click()).clicked() {
                    view_state.expanded_section = if is_overlay_expanded { None } else { Some(SettingsSection::Overlay) };
                }

                if is_overlay_expanded {
                    ui.add_space(16.0);
                    ui.separator();
                    ui.add_space(12.0);

                    let mut state = shared_state.write();

                    ui.horizontal(|ui| {
                        ui.label("Overlay enabled:");
                        ui.add_space(8.0);
                        if ui.checkbox(&mut state.config.overlay.enabled, "").changed() {
                            state.overlay_config.enabled = state.config.overlay.enabled;
                            changed.set(true);
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("Opacity:");
                        ui.add_space(8.0);
                        let mut opacity = state.config.overlay.opacity;
                        if ui.add(egui::Slider::new(&mut opacity, 0.1..=1.0).show_value(true)).changed() {
                            state.config.overlay.opacity = opacity;
                            state.overlay_config.opacity = opacity;
                            changed.set(true);
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("Sound notifications:");
                        ui.add_space(8.0);
                        if ui.checkbox(&mut state.config.overlay.sound_enabled, "").changed() {
                            changed.set(true);
                        }
                    });

                    let sound_enabled = state.config.overlay.sound_enabled;
                    let mut volume = state.config.overlay.sound_volume;
                    drop(state);

                    ui.add_enabled_ui(sound_enabled, |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Sound volume:");
                            ui.add_space(8.0);
                            if ui.add(egui::Slider::new(&mut volume, 0.0..=1.0).show_value(true)).changed() {
                                let mut state = shared_state.write();
                                state.config.overlay.sound_volume = volume;
                                changed.set(true);
                            }
                        });
                    });
                }
            });

        ui.add_space(16.0);

        // Performance Settings
        let is_perf_expanded = view_state.expanded_section == Some(SettingsSection::Performance);
        egui::Frame::none()
            .fill(ThemeColors::BG_MEDIUM)
            .rounding(egui::Rounding::same(8.0))
            .inner_margin(16.0)
            .show(ui, |ui| {
                let header_response = ui.horizontal(|ui| {
                    let arrow = if is_perf_expanded { "v" } else { ">" };
                    ui.label(RichText::new(arrow).size(12.0).color(ThemeColors::TEXT_MUTED));
                    ui.add_space(8.0);
                    ui.heading(RichText::new("Performance").size(16.0));
                }).response;

                if header_response.interact(egui::Sense::click()).clicked() {
                    view_state.expanded_section = if is_perf_expanded { None } else { Some(SettingsSection::Performance) };
                }

                if is_perf_expanded {
                    ui.add_space(16.0);
                    ui.separator();
                    ui.add_space(12.0);

                    let mut state = shared_state.write();

                    ui.horizontal(|ui| {
                        ui.label("Max CPU usage:");
                        ui.add_space(8.0);
                        let mut cpu = state.config.performance.max_cpu_percent as f32;
                        if ui.add(egui::Slider::new(&mut cpu, 1.0..=50.0).suffix("%")).changed() {
                            state.config.performance.max_cpu_percent = cpu as u32;
                            changed.set(true);
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("Max memory:");
                        ui.add_space(8.0);
                        let mut mem = state.config.performance.max_memory_mb as f32;
                        if ui.add(egui::Slider::new(&mut mem, 128.0..=2048.0).suffix(" MB")).changed() {
                            state.config.performance.max_memory_mb = mem as u32;
                            changed.set(true);
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("Idle optimization:");
                        ui.add_space(8.0);
                        if ui.checkbox(&mut state.config.performance.idle_optimization, "").changed() {
                            changed.set(true);
                        }
                    });
                    ui.label(
                        RichText::new("Reduce activity when game is paused or in menu")
                            .size(11.0)
                            .color(ThemeColors::TEXT_MUTED)
                    );
                }
            });

        ui.add_space(24.0);

        // Reset button and auto-save indicator
        ui.horizontal(|ui| {
            if ui.add(
                egui::Button::new("Reset to Defaults")
                    .min_size(egui::vec2(120.0, 36.0))
            ).clicked() {
                let mut state = shared_state.write();
                state.config = crate::config::AppConfig::default();
                state.overlay_config.opacity = state.config.overlay.opacity;
                state.overlay_config.enabled = state.config.overlay.enabled;
                changed.set(true);
            }

            ui.add_space(16.0);

            // Auto-save indicator
            ui.label(
                RichText::new("Settings are saved automatically")
                    .size(12.0)
                    .color(ThemeColors::TEXT_MUTED)
            );
        });
    });

    // Apply changes - triggers auto-save in the dashboard app
    if changed.get() {
        view_state.has_unsaved_changes = true;
    }
}
