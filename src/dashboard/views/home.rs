//! Home view - Status overview and quick controls

use egui::RichText;
use parking_lot::RwLock;
use std::sync::Arc;

use crate::dashboard::components::status_card::{CardStatus, StatusCard};
use crate::dashboard::state::HomeViewState;
use crate::dashboard::theme::{ThemeColors, color_with_alpha};
use crate::shared::{CaptureCommand, OverlayCommand, SharedAppState};

/// Render the home view
pub fn render_home_view(
    ui: &mut egui::Ui,
    _state: &mut HomeViewState,
    shared_state: &Arc<RwLock<SharedAppState>>,
) {
    let app_state = shared_state.read();

    ui.heading(RichText::new("Dashboard").size(24.0).strong());
    ui.add_space(8.0);
    ui.label(
        RichText::new("Monitor and control your GamersToolKit instance")
            .size(14.0)
            .color(ThemeColors::TEXT_SECONDARY)
    );

    ui.add_space(24.0);

    // Status cards row
    ui.horizontal(|ui| {
        // Capture status
        let capture_status = if app_state.runtime.is_capturing {
            CardStatus::Active
        } else {
            CardStatus::Inactive
        };
        let capture_value = if app_state.runtime.is_capturing {
            format!("{:.1} FPS", app_state.runtime.capture_fps)
        } else {
            "Stopped".to_string()
        };
        StatusCard::new("Screen Capture", capture_value, capture_status).show(ui);

        ui.add_space(16.0);

        // Overlay status
        let overlay_status = if app_state.runtime.is_overlay_running {
            if app_state.runtime.overlay_visible {
                CardStatus::Active
            } else {
                CardStatus::Warning
            }
        } else {
            CardStatus::Inactive
        };
        let overlay_value = if app_state.runtime.is_overlay_running {
            if app_state.runtime.overlay_visible {
                "Visible"
            } else {
                "Hidden"
            }
        } else {
            "Stopped"
        };
        StatusCard::new("Overlay", overlay_value, overlay_status).show(ui);

        ui.add_space(16.0);

        // Profile status
        let profile_status = if app_state.active_profile_id.is_some() {
            CardStatus::Active
        } else {
            CardStatus::Inactive
        };
        let profile_value = app_state
            .active_profile()
            .map(|p| p.name.clone())
            .unwrap_or_else(|| "None".to_string());
        StatusCard::new("Active Profile", profile_value, profile_status).show(ui);
    });

    ui.add_space(32.0);

    // Quick actions section
    ui.horizontal(|ui| {
        ui.heading(RichText::new("Quick Actions").size(18.0));
    });

    ui.add_space(16.0);

    drop(app_state); // Release read lock before potential write operations

    ui.horizontal(|ui| {
        // Start/Stop Capture button
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
                .min_size(egui::vec2(120.0, 36.0))
        ).clicked() {
            let mut state = shared_state.write();
            state.runtime.capture_command = Some(if is_capturing {
                CaptureCommand::Stop
            } else {
                CaptureCommand::Start
            });
        }

        ui.add_space(12.0);

        // Start/Stop Overlay button
        let is_overlay_running = shared_state.read().runtime.is_overlay_running;
        let overlay_btn_text = if is_overlay_running { "Stop Overlay" } else { "Start Overlay" };
        let overlay_btn_color = if is_overlay_running {
            ThemeColors::ACCENT_ERROR
        } else {
            ThemeColors::ACCENT_SUCCESS
        };

        if ui.add(
            egui::Button::new(RichText::new(overlay_btn_text).color(egui::Color32::WHITE))
                .fill(overlay_btn_color)
                .min_size(egui::vec2(120.0, 36.0))
        ).clicked() {
            let mut state = shared_state.write();
            state.runtime.overlay_command = Some(if is_overlay_running {
                OverlayCommand::Stop
            } else {
                OverlayCommand::Start
            });
        }

        ui.add_space(12.0);

        // Show/Hide Overlay button (only when overlay is running)
        if is_overlay_running {
            let overlay_visible = shared_state.read().runtime.overlay_visible;
            let visibility_btn_text = if overlay_visible { "Hide Overlay" } else { "Show Overlay" };

            if ui.add(
                egui::Button::new(visibility_btn_text)
                    .min_size(egui::vec2(120.0, 36.0))
            ).clicked() {
                let mut state = shared_state.write();
                state.runtime.overlay_command = Some(OverlayCommand::ToggleVisibility);
            }

            ui.add_space(12.0);
        }

        // Test Tip button (only when overlay is running)
        if is_overlay_running {
            if ui.add(
                egui::Button::new("Send Test Tip")
                    .min_size(egui::vec2(120.0, 36.0))
            ).clicked() {
                let mut state = shared_state.write();
                state.runtime.send_test_tip = true;
            }
        }
    });

    ui.add_space(32.0);

    // Current configuration summary
    ui.heading(RichText::new("Configuration Summary").size(18.0));
    ui.add_space(12.0);

    let app_state = shared_state.read();

    egui::Grid::new("config_summary")
        .num_columns(2)
        .spacing([40.0, 8.0])
        .show(ui, |ui| {
            ui.label(RichText::new("Capture Target:").color(ThemeColors::TEXT_MUTED));
            ui.label(app_state.runtime.current_capture_target.as_deref().unwrap_or("Not set"));
            ui.end_row();

            ui.label(RichText::new("Max FPS:").color(ThemeColors::TEXT_MUTED));
            ui.label(format!("{}", app_state.config.capture.max_fps));
            ui.end_row();

            ui.label(RichText::new("Overlay Opacity:").color(ThemeColors::TEXT_MUTED));
            ui.label(format!("{:.0}%", app_state.overlay_config.opacity * 100.0));
            ui.end_row();

            ui.label(RichText::new("Tips Displayed:").color(ThemeColors::TEXT_MUTED));
            ui.label(format!("{}", app_state.runtime.tips_displayed));
            ui.end_row();
        });

    // Error display
    if let Some(error) = &app_state.runtime.last_error {
        ui.add_space(24.0);
        egui::Frame::none()
            .fill(color_with_alpha(ThemeColors::ACCENT_ERROR, 51)) // ~0.2 alpha
            .rounding(egui::Rounding::same(6.0))
            .inner_margin(12.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Error:").color(ThemeColors::ACCENT_ERROR).strong());
                    ui.label(RichText::new(error).color(ThemeColors::TEXT_PRIMARY));
                });
            });
    }
}
