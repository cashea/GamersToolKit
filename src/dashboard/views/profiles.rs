//! Profiles view - Game profile management

use egui::RichText;
use parking_lot::RwLock;
use std::sync::Arc;

use crate::dashboard::state::{ProfilesViewState, ProfileAction};
use crate::dashboard::theme::{ThemeColors, color_with_alpha};
use crate::shared::SharedAppState;
use crate::storage::profiles::GameProfile;

/// Render the profiles view
pub fn render_profiles_view(
    ui: &mut egui::Ui,
    view_state: &mut ProfilesViewState,
    shared_state: &Arc<RwLock<SharedAppState>>,
) {
    ui.heading(RichText::new("Game Profiles").size(24.0).strong());
    ui.add_space(8.0);
    ui.label(
        RichText::new("Manage game-specific detection and rules")
            .size(14.0)
            .color(ThemeColors::TEXT_SECONDARY)
    );

    ui.add_space(24.0);

    // Toolbar
    ui.horizontal(|ui| {
        // Search
        ui.label("Search:");
        ui.add_space(8.0);
        ui.add(
            egui::TextEdit::singleline(&mut view_state.search_query)
                .hint_text("Filter profiles...")
                .desired_width(200.0)
        );

        ui.add_space(16.0);

        // Create new profile button
        if ui.add(
            egui::Button::new(
                RichText::new("+ New Profile")
                    .color(egui::Color32::WHITE)
            )
            .fill(ThemeColors::ACCENT_PRIMARY)
        ).clicked() {
            view_state.show_create_dialog = true;
            view_state.new_profile_name.clear();
            view_state.new_profile_executable.clear();
        }
    });

    ui.add_space(16.0);

    // Content area
    ui.horizontal(|ui| {
        // Left side: Profile list
        egui::Frame::none()
            .fill(ThemeColors::BG_MEDIUM)
            .rounding(egui::Rounding::same(8.0))
            .inner_margin(12.0)
            .show(ui, |ui| {
                ui.set_min_width(280.0);
                ui.set_min_height(400.0);

                let state = shared_state.read();
                let filter = view_state.search_query.to_lowercase();

                let filtered_profiles: Vec<_> = state.profiles
                    .iter()
                    .filter(|p| filter.is_empty() || p.name.to_lowercase().contains(&filter))
                    .collect();

                if filtered_profiles.is_empty() {
                    ui.centered_and_justified(|ui| {
                        if state.profiles.is_empty() {
                            ui.vertical_centered(|ui| {
                                ui.add_space(60.0);
                                ui.label(
                                    RichText::new("No profiles yet")
                                        .size(16.0)
                                        .color(ThemeColors::TEXT_MUTED)
                                );
                                ui.add_space(8.0);
                                ui.label(
                                    RichText::new("Create your first game profile to get started")
                                        .size(12.0)
                                        .color(ThemeColors::TEXT_MUTED)
                                );
                            });
                        } else {
                            ui.label(
                                RichText::new("No profiles match your search")
                                    .color(ThemeColors::TEXT_MUTED)
                            );
                        }
                    });
                } else {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for profile in filtered_profiles {
                            let is_selected = view_state.selected_profile_id.as_ref() == Some(&profile.id);
                            let is_active = state.active_profile_id.as_ref() == Some(&profile.id);

                            render_profile_card(ui, profile, is_selected, is_active, view_state);
                            ui.add_space(8.0);
                        }
                    });
                }
            });

        ui.add_space(16.0);

        // Right side: Profile details
        egui::Frame::none()
            .fill(ThemeColors::BG_MEDIUM)
            .rounding(egui::Rounding::same(8.0))
            .inner_margin(16.0)
            .show(ui, |ui| {
                ui.set_min_width(350.0);
                ui.set_min_height(400.0);

                let state = shared_state.read();

                if let Some(profile_id) = &view_state.selected_profile_id {
                    if let Some(profile) = state.profiles.iter().find(|p| &p.id == profile_id) {
                        render_profile_details(ui, profile, view_state, &state);
                    } else {
                        render_no_selection(ui);
                    }
                } else {
                    render_no_selection(ui);
                }
            });
    });

    // Create profile dialog
    if view_state.show_create_dialog {
        render_create_dialog(ui, view_state, shared_state);
    }

    // Delete confirmation dialog
    if view_state.show_delete_confirm {
        render_delete_confirm_dialog(ui, view_state, shared_state);
    }
}

/// Render a profile card in the list
fn render_profile_card(
    ui: &mut egui::Ui,
    profile: &GameProfile,
    is_selected: bool,
    is_active: bool,
    view_state: &mut ProfilesViewState,
) {
    let bg_color = if is_selected {
        color_with_alpha(ThemeColors::ACCENT_PRIMARY, 51) // ~0.2 alpha
    } else {
        ThemeColors::BG_LIGHT
    };

    let response = egui::Frame::none()
        .fill(bg_color)
        .rounding(egui::Rounding::same(6.0))
        .inner_margin(12.0)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&profile.name).strong());
                        if is_active {
                            ui.label(
                                RichText::new("Active")
                                    .size(10.0)
                                    .color(ThemeColors::ACCENT_SUCCESS)
                            );
                        }
                    });

                    ui.label(
                        RichText::new(format!("v{}", profile.version))
                            .size(11.0)
                            .color(ThemeColors::TEXT_MUTED)
                    );

                    let exe_count = profile.executables.len();
                    let rule_count = profile.rules.len();
                    ui.label(
                        RichText::new(format!("{} exe(s), {} rule(s)", exe_count, rule_count))
                            .size(11.0)
                            .color(ThemeColors::TEXT_MUTED)
                    );
                });
            });
        }).response;

    if response.interact(egui::Sense::click()).clicked() {
        view_state.selected_profile_id = Some(profile.id.clone());
    }
}

/// Render profile details panel
fn render_profile_details(
    ui: &mut egui::Ui,
    profile: &GameProfile,
    view_state: &mut ProfilesViewState,
    state: &SharedAppState,
) {
    ui.heading(RichText::new(&profile.name).size(18.0));
    ui.add_space(4.0);
    ui.label(
        RichText::new(format!("Version {}", profile.version))
            .size(12.0)
            .color(ThemeColors::TEXT_MUTED)
    );

    ui.add_space(16.0);

    // Executables
    ui.label(RichText::new("Executables").strong());
    ui.add_space(4.0);
    for exe in &profile.executables {
        ui.horizontal(|ui| {
            ui.label(RichText::new("-").color(ThemeColors::TEXT_MUTED));
            ui.label(exe);
        });
    }
    if profile.executables.is_empty() {
        ui.label(
            RichText::new("No executables defined")
                .size(12.0)
                .color(ThemeColors::TEXT_MUTED)
        );
    }

    ui.add_space(16.0);

    // OCR Regions
    ui.label(RichText::new("OCR Regions").strong());
    ui.add_space(4.0);
    ui.label(
        RichText::new(format!("{} region(s) defined", profile.ocr_regions.len()))
            .size(12.0)
            .color(ThemeColors::TEXT_SECONDARY)
    );

    ui.add_space(16.0);

    // Rules
    ui.label(RichText::new("Rules").strong());
    ui.add_space(4.0);
    for rule in &profile.rules {
        ui.horizontal(|ui| {
            let status_color = if rule.enabled {
                ThemeColors::ACCENT_SUCCESS
            } else {
                ThemeColors::TEXT_MUTED
            };
            ui.label(RichText::new(if rule.enabled { "[ON]" } else { "[OFF]" }).color(status_color).size(10.0));
            ui.label(&rule.name);
        });
    }
    if profile.rules.is_empty() {
        ui.label(
            RichText::new("No rules defined")
                .size(12.0)
                .color(ThemeColors::TEXT_MUTED)
        );
    }

    ui.add_space(24.0);

    // Action buttons
    ui.horizontal(|ui| {
        let is_active = state.active_profile_id.as_ref() == Some(&profile.id);

        if is_active {
            if ui.add(
                egui::Button::new("Deactivate")
                    .min_size(egui::vec2(100.0, 32.0))
            ).clicked() {
                view_state.pending_action = Some(ProfileAction::Deactivate);
            }
        } else {
            if ui.add(
                egui::Button::new(
                    RichText::new("Activate")
                        .color(egui::Color32::WHITE)
                )
                .fill(ThemeColors::ACCENT_SUCCESS)
                .min_size(egui::vec2(100.0, 32.0))
            ).clicked() {
                view_state.pending_action = Some(ProfileAction::Activate(profile.id.clone()));
            }
        }

        ui.add_space(8.0);

        if ui.add(
            egui::Button::new(
                RichText::new("Delete")
                    .color(egui::Color32::WHITE)
            )
            .fill(ThemeColors::ACCENT_ERROR)
            .min_size(egui::vec2(80.0, 32.0))
        ).clicked() {
            view_state.show_delete_confirm = true;
        }
    });
}

/// Render the "no selection" placeholder
fn render_no_selection(ui: &mut egui::Ui) {
    ui.centered_and_justified(|ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(100.0);
            ui.label(
                RichText::new("Select a profile")
                    .size(16.0)
                    .color(ThemeColors::TEXT_MUTED)
            );
            ui.add_space(8.0);
            ui.label(
                RichText::new("Choose a profile from the list to view details")
                    .size(12.0)
                    .color(ThemeColors::TEXT_MUTED)
            );
        });
    });
}

/// Render the create profile dialog
fn render_create_dialog(
    ui: &mut egui::Ui,
    view_state: &mut ProfilesViewState,
    shared_state: &Arc<RwLock<SharedAppState>>,
) {
    egui::Window::new("Create New Profile")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .show(ui.ctx(), |ui| {
            ui.set_min_width(300.0);

            ui.horizontal(|ui| {
                ui.label("Profile Name:");
                ui.text_edit_singleline(&mut view_state.new_profile_name);
            });

            ui.add_space(8.0);

            ui.horizontal(|ui| {
                ui.label("Executable:");
                ui.text_edit_singleline(&mut view_state.new_profile_executable);
            });

            ui.add_space(16.0);

            ui.horizontal(|ui| {
                if ui.button("Cancel").clicked() {
                    view_state.show_create_dialog = false;
                }

                ui.add_space(8.0);

                let can_create = !view_state.new_profile_name.is_empty();
                ui.add_enabled_ui(can_create, |ui| {
                    if ui.add(
                        egui::Button::new(
                            RichText::new("Create")
                                .color(egui::Color32::WHITE)
                        )
                        .fill(ThemeColors::ACCENT_PRIMARY)
                    ).clicked() {
                        let new_profile = GameProfile {
                            id: format!("profile_{}", std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_secs()),
                            name: view_state.new_profile_name.clone(),
                            executables: if view_state.new_profile_executable.is_empty() {
                                vec![]
                            } else {
                                vec![view_state.new_profile_executable.clone()]
                            },
                            version: "1.0.0".to_string(),
                            ocr_regions: vec![],
                            templates: vec![],
                            rules: vec![],
                            labeled_regions: vec![],
                        };

                        let mut state = shared_state.write();
                        state.add_profile(new_profile.clone());
                        view_state.selected_profile_id = Some(new_profile.id);
                        view_state.show_create_dialog = false;
                    }
                });
            });
        });
}

/// Render the delete confirmation dialog
fn render_delete_confirm_dialog(
    ui: &mut egui::Ui,
    view_state: &mut ProfilesViewState,
    shared_state: &Arc<RwLock<SharedAppState>>,
) {
    egui::Window::new("Confirm Delete")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .show(ui.ctx(), |ui| {
            ui.label("Are you sure you want to delete this profile?");
            ui.label(
                RichText::new("This action cannot be undone.")
                    .color(ThemeColors::ACCENT_WARNING)
            );

            ui.add_space(16.0);

            ui.horizontal(|ui| {
                if ui.button("Cancel").clicked() {
                    view_state.show_delete_confirm = false;
                }

                ui.add_space(8.0);

                if ui.add(
                    egui::Button::new(
                        RichText::new("Delete")
                            .color(egui::Color32::WHITE)
                    )
                    .fill(ThemeColors::ACCENT_ERROR)
                ).clicked() {
                    if let Some(profile_id) = view_state.selected_profile_id.take() {
                        let mut state = shared_state.write();
                        state.remove_profile(&profile_id);
                    }
                    view_state.show_delete_confirm = false;
                }
            });
        });
}
