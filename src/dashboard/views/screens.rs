//! Screen Recognition View
//!
//! UI for managing screen definitions and screen recognition settings.

use std::sync::Arc;
use egui::{RichText, ScrollArea};
use parking_lot::RwLock;

use crate::dashboard::state::{DashboardState, ScreensViewState};
use crate::dashboard::theme::ThemeColors;
use crate::shared::SharedAppState;
use crate::storage::profiles::{ScreenDefinition, ScreenMatchMode, ScreenAnchor, AnchorType};

/// Render the screens view
pub fn render_screens_view(
    ui: &mut egui::Ui,
    state: &mut DashboardState,
    shared_state: &Arc<RwLock<SharedAppState>>,
) {
    let view_state = &mut state.screens;

    ui.horizontal(|ui| {
        ui.heading(RichText::new("Screen Recognition").color(ThemeColors::TEXT_PRIMARY));
        ui.add_space(16.0);

        // Recognition toggle
        let shared = shared_state.read();
        let is_enabled = shared.active_profile()
            .map(|p| p.screen_recognition_enabled)
            .unwrap_or(false);
        drop(shared);

        let toggle_text = if is_enabled { "Enabled" } else { "Disabled" };
        let toggle_color = if is_enabled { ThemeColors::ACCENT_SUCCESS } else { ThemeColors::TEXT_MUTED };

        if ui.button(RichText::new(toggle_text).color(toggle_color)).clicked() {
            let mut shared = shared_state.write();
            let active_id = shared.active_profile_id.clone();
            if let Some(profile) = shared.profiles.iter_mut()
                .find(|p| active_id.as_ref() == Some(&p.id))
            {
                profile.screen_recognition_enabled = !profile.screen_recognition_enabled;
                view_state.screens_dirty = true;
            }
        }
    });

    ui.add_space(8.0);

    // Show current detected screen
    {
        let shared = shared_state.read();
        if let Some(ref screen_match) = shared.runtime.current_screen {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Current Screen:").color(ThemeColors::TEXT_SECONDARY));
                ui.label(RichText::new(&screen_match.screen_name).color(ThemeColors::ACCENT_PRIMARY).strong());
                ui.label(RichText::new(format!("({:.0}%)", screen_match.confidence * 100.0))
                    .color(ThemeColors::TEXT_MUTED));
            });
        } else {
            ui.label(RichText::new("No screen detected").color(ThemeColors::TEXT_MUTED));
        }
    }

    ui.add_space(16.0);
    ui.separator();
    ui.add_space(8.0);

    // Main content area - split into tree and details
    let available_height = ui.available_height();

    ui.horizontal(|ui| {
        // Left panel: Screen hierarchy tree
        ui.vertical(|ui| {
            ui.set_min_width(250.0);
            ui.set_max_width(300.0);
            ui.set_min_height(available_height);

            ui.horizontal(|ui| {
                ui.label(RichText::new("Screens").color(ThemeColors::TEXT_PRIMARY).strong());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let has_active_profile = {
                        let shared = shared_state.read();
                        shared.active_profile_id.is_some()
                    };

                    ui.add_enabled_ui(has_active_profile, |ui| {
                        let tooltip = if has_active_profile {
                            "Add new screen"
                        } else {
                            "Select a profile first to add screens"
                        };
                        if ui.button("+").on_hover_text(tooltip).clicked() {
                            view_state.show_add_dialog = true;
                            view_state.new_screen_name.clear();
                            view_state.new_screen_parent_id = None;
                        }
                    });
                });
            });

            ui.add_space(8.0);

            ScrollArea::vertical()
                .id_salt("screen_tree")
                .show(ui, |ui| {
                    render_screen_tree(ui, view_state, shared_state);
                });
        });

        ui.separator();

        // Right panel: Selected screen details
        ui.vertical(|ui| {
            ui.set_min_width(350.0);
            ui.set_min_height(available_height);

            if let Some(ref screen_id) = view_state.selected_screen_id.clone() {
                render_screen_details(ui, screen_id, view_state, shared_state);
            } else {
                ui.centered_and_justified(|ui| {
                    ui.label(RichText::new("Select a screen to view details")
                        .color(ThemeColors::TEXT_MUTED));
                });
            }
        });
    });

    // Add screen dialog
    if view_state.show_add_dialog {
        render_add_screen_dialog(ui, view_state, shared_state);
    }

    // Delete confirmation dialog
    if view_state.show_delete_confirm {
        render_delete_confirm_dialog(ui, view_state, shared_state);
    }

    // Text anchor confirmation dialog
    if view_state.pending_text_for_anchor.is_some() {
        render_text_anchor_dialog(ui, view_state, shared_state);
    }

    // Error message
    if let Some(ref error) = view_state.error_message.clone() {
        ui.add_space(8.0);
        ui.label(RichText::new(error).color(ThemeColors::ACCENT_ERROR));
    }
}

/// Render the screen hierarchy tree
fn render_screen_tree(
    ui: &mut egui::Ui,
    view_state: &mut ScreensViewState,
    shared_state: &Arc<RwLock<SharedAppState>>,
) {
    let shared = shared_state.read();
    let has_active_profile = shared.active_profile_id.is_some();
    let screens = shared.active_profile()
        .map(|p| p.screens.clone())
        .unwrap_or_default();
    drop(shared);

    if !has_active_profile {
        ui.label(RichText::new("No profile selected").color(ThemeColors::TEXT_MUTED).italics());
        ui.add_space(8.0);
        ui.label(RichText::new("Go to Profiles and activate a profile first")
            .color(ThemeColors::TEXT_MUTED).small());
        return;
    }

    if screens.is_empty() {
        ui.label(RichText::new("No screens defined").color(ThemeColors::TEXT_MUTED).italics());
        ui.add_space(8.0);
        ui.label(RichText::new("Click '+' to add a screen").color(ThemeColors::TEXT_MUTED).small());
        return;
    }

    // Hint for drag and drop
    if screens.len() > 1 {
        ui.label(RichText::new("Drag to reorder priority").color(ThemeColors::TEXT_MUTED).small());
        ui.add_space(4.0);
    }

    // Build tree structure - find root screens first, sorted by priority (higher first)
    let mut root_screens: Vec<_> = screens.iter()
        .filter(|s| s.parent_id.is_none())
        .collect();
    root_screens.sort_by(|a, b| b.priority.cmp(&a.priority));

    // Track drop operations to perform after rendering
    let mut drop_action: Option<(String, String, bool)> = None;

    for screen in &root_screens {
        if let Some(action) = render_screen_tree_node(ui, screen, &screens, view_state, 0) {
            drop_action = Some(action);
        }
    }

    // Handle drop at the end (after all screens - lowest priority)
    if view_state.dragging_screen_id.is_some() {
        let drop_zone_response = ui.allocate_response(
            egui::vec2(ui.available_width(), 20.0),
            egui::Sense::hover(),
        );

        if drop_zone_response.hovered() && ui.input(|i| i.pointer.any_released()) {
            // Drop at the end - set to lowest priority
            if let Some(dragged_id) = view_state.dragging_screen_id.take() {
                reorder_screen_priority(shared_state, &dragged_id, None, view_state);
            }
        }
    }

    // Handle mouse release to end drag
    if ui.input(|i| i.pointer.any_released()) {
        if let Some((dragged_id, target_id, before)) = drop_action {
            reorder_screen_priority(shared_state, &dragged_id, Some((&target_id, before)), view_state);
        }
        view_state.dragging_screen_id = None;
        view_state.drop_target_screen_id = None;
    }
}

/// Render a single screen tree node
/// Returns Some((dragged_id, target_id, before)) if a drop should occur
fn render_screen_tree_node(
    ui: &mut egui::Ui,
    screen: &ScreenDefinition,
    all_screens: &[ScreenDefinition],
    view_state: &mut ScreensViewState,
    depth: usize,
) -> Option<(String, String, bool)> {
    let is_selected = view_state.selected_screen_id.as_ref() == Some(&screen.id);
    let is_being_dragged = view_state.dragging_screen_id.as_ref() == Some(&screen.id);
    let is_drop_target = view_state.drop_target_screen_id.as_ref() == Some(&screen.id);
    let indent = depth as f32 * 16.0;

    let mut drop_action: Option<(String, String, bool)> = None;

    // Only show root-level screens for drag reordering (no parent)
    let is_root = screen.parent_id.is_none();

    // Draw drop indicator above if this is the drop target and we're dropping before
    if is_drop_target && view_state.drop_before_target && is_root {
        ui.horizontal(|ui| {
            ui.add_space(indent);
            let rect = ui.available_rect_before_wrap();
            let indicator_rect = egui::Rect::from_min_size(
                rect.min,
                egui::vec2(rect.width().min(200.0), 2.0),
            );
            ui.painter().rect_filled(indicator_rect, 0.0, ThemeColors::ACCENT_PRIMARY);
        });
    }

    let response = ui.horizontal(|ui| {
        ui.add_space(indent);

        // Drag handle for root screens
        if is_root {
            let drag_handle_text = RichText::new("≡")
                .color(if is_being_dragged { ThemeColors::ACCENT_PRIMARY } else { ThemeColors::TEXT_MUTED });

            // Use a Button styled as a label for drag functionality
            let drag_handle = ui.add(
                egui::Button::new(drag_handle_text)
                    .frame(false)
                    .sense(egui::Sense::drag())
            );

            // Start drag on drag handle
            if drag_handle.drag_started() {
                view_state.dragging_screen_id = Some(screen.id.clone());
            }

            // Change cursor to indicate draggable
            if drag_handle.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
            }
            if is_being_dragged {
                ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
            }
        }

        // Find children to determine if we show expand indicator
        let children: Vec<_> = all_screens.iter()
            .filter(|s| s.parent_id.as_ref() == Some(&screen.id))
            .collect();

        let has_children = !children.is_empty();
        if has_children {
            ui.label(RichText::new("-").color(ThemeColors::TEXT_MUTED));
        } else {
            ui.add_space(12.0);
        }

        // Screen icon based on match mode
        let icon = match screen.match_mode {
            ScreenMatchMode::FullScreenshot => "[F]",
            ScreenMatchMode::Anchors => "[A]",
        };

        // Adjust color based on drag state
        let text_color = if is_being_dragged {
            ThemeColors::ACCENT_PRIMARY
        } else if is_selected {
            ThemeColors::ACCENT_PRIMARY
        } else if screen.enabled {
            ThemeColors::TEXT_PRIMARY
        } else {
            ThemeColors::TEXT_MUTED
        };

        ui.label(RichText::new(icon).color(ThemeColors::TEXT_MUTED).small());

        // Priority indicator
        ui.label(RichText::new(format!("({})", screen.priority)).color(ThemeColors::TEXT_MUTED).small());

        if ui.selectable_label(is_selected, RichText::new(&screen.name).color(text_color)).clicked() {
            if view_state.dragging_screen_id.is_none() {
                view_state.selected_screen_id = Some(screen.id.clone());
            }
        }
    });

    // Handle drop target detection for root screens
    if is_root {
        if let Some(ref dragged_id) = view_state.dragging_screen_id.clone() {
            if dragged_id != &screen.id && response.response.hovered() {
                // Determine if we're in the top or bottom half
                if let Some(pointer_pos) = ui.input(|i| i.pointer.hover_pos()) {
                    let rect = response.response.rect;
                    let mid_y = rect.center().y;
                    let before = pointer_pos.y < mid_y;

                    view_state.drop_target_screen_id = Some(screen.id.clone());
                    view_state.drop_before_target = before;

                    // Check if we should trigger a drop
                    if ui.input(|i| i.pointer.any_released()) {
                        drop_action = Some((dragged_id.clone(), screen.id.clone(), before));
                    }
                }
            }
        }
    }

    // Draw drop indicator below if this is the drop target and we're dropping after
    if is_drop_target && !view_state.drop_before_target && is_root {
        ui.horizontal(|ui| {
            ui.add_space(indent);
            let rect = ui.available_rect_before_wrap();
            let indicator_rect = egui::Rect::from_min_size(
                rect.min,
                egui::vec2(rect.width().min(200.0), 2.0),
            );
            ui.painter().rect_filled(indicator_rect, 0.0, ThemeColors::ACCENT_PRIMARY);
        });
    }

    // Render children (sorted by priority)
    let mut children: Vec<_> = all_screens.iter()
        .filter(|s| s.parent_id.as_ref() == Some(&screen.id))
        .collect();
    children.sort_by(|a, b| b.priority.cmp(&a.priority));

    for child in children {
        if let Some(action) = render_screen_tree_node(ui, child, all_screens, view_state, depth + 1) {
            drop_action = Some(action);
        }
    }

    drop_action
}

/// Reorder screen priorities when a screen is dropped
fn reorder_screen_priority(
    shared_state: &Arc<RwLock<SharedAppState>>,
    dragged_id: &str,
    target: Option<(&str, bool)>, // (target_id, before)
    view_state: &mut ScreensViewState,
) {
    let mut shared = shared_state.write();
    let active_id = shared.active_profile_id.clone();

    if let Some(profile) = shared.profiles.iter_mut().find(|p| active_id.as_ref() == Some(&p.id)) {
        // Get only root screens (no parent) sorted by priority descending
        let mut root_screens: Vec<_> = profile.screens.iter()
            .filter(|s| s.parent_id.is_none())
            .map(|s| (s.id.clone(), s.priority))
            .collect();
        root_screens.sort_by(|a, b| b.1.cmp(&a.1));

        // Find current position of dragged screen
        let dragged_pos = root_screens.iter().position(|(id, _)| id == dragged_id);
        if dragged_pos.is_none() {
            return;
        }
        let dragged_pos = dragged_pos.unwrap();

        // Remove dragged screen from list
        let dragged_screen = root_screens.remove(dragged_pos);

        // Insert at new position
        let insert_pos = if let Some((target_id, before)) = target {
            if let Some(target_pos) = root_screens.iter().position(|(id, _)| id == target_id) {
                if before {
                    target_pos
                } else {
                    target_pos + 1
                }
            } else {
                root_screens.len() // Default to end if target not found
            }
        } else {
            root_screens.len() // Drop at end
        };

        root_screens.insert(insert_pos.min(root_screens.len()), dragged_screen);

        // Reassign priorities based on new order (higher index = lower priority)
        // We want the first item to have the highest priority
        let base_priority = 100u32;
        for (idx, (screen_id, _)) in root_screens.iter().enumerate() {
            if let Some(screen) = profile.screens.iter_mut().find(|s| &s.id == screen_id) {
                screen.priority = base_priority.saturating_sub(idx as u32 * 10);
            }
        }

        view_state.screens_dirty = true;
    }
}

/// Render details for a selected screen
fn render_screen_details(
    ui: &mut egui::Ui,
    screen_id: &str,
    view_state: &mut ScreensViewState,
    shared_state: &Arc<RwLock<SharedAppState>>,
) {
    // Get screen data for display
    let shared = shared_state.read();
    let screen = shared.active_profile()
        .and_then(|p| p.screens.iter().find(|s| s.id == screen_id).cloned());
    drop(shared);

    let Some(mut screen) = screen else {
        ui.label(RichText::new("Screen not found").color(ThemeColors::ACCENT_ERROR));
        return;
    };

    // Editable name
    ui.horizontal(|ui| {
        ui.label(RichText::new("Name:").color(ThemeColors::TEXT_SECONDARY));
        let response = ui.text_edit_singleline(&mut screen.name);
        if response.changed() {
            update_screen_field(shared_state, screen_id, |s| s.name = screen.name.clone());
            view_state.screens_dirty = true;
        }
    });

    ui.add_space(8.0);

    // Basic info grid with editable fields
    egui::Grid::new("screen_details_grid")
        .num_columns(2)
        .spacing([8.0, 8.0])
        .show(ui, |ui| {
            // ID (read-only)
            ui.label(RichText::new("ID:").color(ThemeColors::TEXT_SECONDARY));
            ui.label(RichText::new(&screen.id).color(ThemeColors::TEXT_MUTED).small());
            ui.end_row();

            // Enabled toggle
            ui.label(RichText::new("Enabled:").color(ThemeColors::TEXT_SECONDARY));
            let mut enabled = screen.enabled;
            if ui.checkbox(&mut enabled, "").changed() {
                update_screen_field(shared_state, screen_id, |s| s.enabled = enabled);
                view_state.screens_dirty = true;
            }
            ui.end_row();

            // Match threshold slider
            ui.label(RichText::new("Threshold:").color(ThemeColors::TEXT_SECONDARY));
            ui.horizontal(|ui| {
                let mut threshold = screen.match_threshold;
                let slider = egui::Slider::new(&mut threshold, 0.5..=1.0)
                    .fixed_decimals(2)
                    .text("")
                    .custom_formatter(|v, _| format!("{:.0}%", v * 100.0));
                if ui.add(slider).changed() {
                    update_screen_field(shared_state, screen_id, |s| s.match_threshold = threshold);
                    view_state.screens_dirty = true;
                }
            });
            ui.end_row();

            // Priority
            ui.label(RichText::new("Priority:").color(ThemeColors::TEXT_SECONDARY));
            let mut priority = screen.priority as i32;
            if ui.add(egui::DragValue::new(&mut priority).range(0..=100)).changed() {
                update_screen_field(shared_state, screen_id, |s| s.priority = priority as u32);
                view_state.screens_dirty = true;
            }
            ui.end_row();

            // Match mode
            ui.label(RichText::new("Match Mode:").color(ThemeColors::TEXT_SECONDARY));
            let mut match_mode = screen.match_mode.clone();
            egui::ComboBox::from_id_salt("screen_match_mode")
                .selected_text(match match_mode {
                    ScreenMatchMode::Anchors => "Anchors",
                    ScreenMatchMode::FullScreenshot => "Full Screenshot",
                })
                .show_ui(ui, |ui| {
                    if ui.selectable_value(&mut match_mode, ScreenMatchMode::Anchors, "Anchors").changed() {
                        update_screen_field(shared_state, screen_id, |s| s.match_mode = match_mode.clone());
                        view_state.screens_dirty = true;
                    }
                    if ui.selectable_value(&mut match_mode, ScreenMatchMode::FullScreenshot, "Full Screenshot").changed() {
                        update_screen_field(shared_state, screen_id, |s| s.match_mode = match_mode.clone());
                        view_state.screens_dirty = true;
                    }
                });
            ui.end_row();
        });

    ui.add_space(16.0);
    ui.separator();
    ui.add_space(8.0);

    // Anchors section
    ui.horizontal(|ui| {
        ui.label(RichText::new("Anchors").color(ThemeColors::TEXT_PRIMARY).strong());
        ui.label(RichText::new(format!("({})", screen.anchors.len())).color(ThemeColors::TEXT_MUTED));
    });

    ui.add_space(4.0);

    if screen.anchors.is_empty() {
        ui.label(RichText::new("No anchors defined").color(ThemeColors::TEXT_MUTED).italics());
    } else {
        let mut anchor_to_delete: Option<String> = None;
        for anchor in &screen.anchors {
            if let Some(id) = render_anchor_item_with_controls(ui, anchor, screen_id, view_state, shared_state) {
                anchor_to_delete = Some(id);
            }
        }
        // Handle anchor deletion outside the loop
        if let Some(anchor_id) = anchor_to_delete {
            let mut shared = shared_state.write();
            let active_id = shared.active_profile_id.clone();
            if let Some(profile) = shared.profiles.iter_mut().find(|p| active_id.as_ref() == Some(&p.id)) {
                if let Some(screen) = profile.screens.iter_mut().find(|s| s.id == screen_id) {
                    screen.anchors.retain(|a| a.id != anchor_id);
                    view_state.screens_dirty = true;
                }
            }
        }
    }

    ui.add_space(8.0);

    // Actions
    ui.horizontal(|ui| {
        if ui.button("Add Visual Anchor").clicked() {
            view_state.pending_anchor_capture = Some(screen.id.clone());
        }
        if ui.button("Add Text Anchor").clicked() {
            view_state.pending_text_anchor_capture = Some(screen.id.clone());
        }
    });

    ui.add_space(16.0);

    // Zone overrides section
    if !screen.ocr_zone_overrides.is_empty() {
        ui.label(RichText::new("Zone Overrides").color(ThemeColors::TEXT_PRIMARY).strong());
        ui.add_space(4.0);
        for override_item in &screen.ocr_zone_overrides {
            ui.horizontal(|ui| {
                let status = if override_item.enabled { "Enable" } else { "Disable" };
                ui.label(RichText::new(format!("{}: {}", status, override_item.zone_id))
                    .color(ThemeColors::TEXT_SECONDARY));
            });
        }
    }

    ui.add_space(16.0);

    // Delete button
    ui.horizontal(|ui| {
        if ui.button(RichText::new("Delete Screen").color(ThemeColors::ACCENT_ERROR)).clicked() {
            view_state.show_delete_confirm = true;
        }
    });
}

/// Helper to update a single field on a screen
fn update_screen_field<F>(shared_state: &Arc<RwLock<SharedAppState>>, screen_id: &str, update: F)
where
    F: FnOnce(&mut ScreenDefinition),
{
    let mut shared = shared_state.write();
    let active_id = shared.active_profile_id.clone();
    if let Some(profile) = shared.profiles.iter_mut().find(|p| active_id.as_ref() == Some(&p.id)) {
        if let Some(screen) = profile.screens.iter_mut().find(|s| s.id == screen_id) {
            update(screen);
        }
    }
}

/// Render anchor item with edit controls, returns anchor ID if delete was clicked
fn render_anchor_item_with_controls(
    ui: &mut egui::Ui,
    anchor: &ScreenAnchor,
    screen_id: &str,
    view_state: &mut ScreensViewState,
    shared_state: &Arc<RwLock<SharedAppState>>,
) -> Option<String> {
    let mut delete_clicked = false;

    ui.horizontal(|ui| {
        // Type icon
        let icon = match anchor.anchor_type {
            AnchorType::Visual => "[V]",
            AnchorType::Text => "[T]",
        };
        ui.label(RichText::new(icon).color(ThemeColors::TEXT_MUTED));

        // Anchor ID
        ui.label(RichText::new(&anchor.id).color(ThemeColors::TEXT_PRIMARY));

        // Required indicator
        let mut required = anchor.required;
        if ui.checkbox(&mut required, "Required").changed() {
            let mut shared = shared_state.write();
            let active_id = shared.active_profile_id.clone();
            if let Some(profile) = shared.profiles.iter_mut().find(|p| active_id.as_ref() == Some(&p.id)) {
                if let Some(screen) = profile.screens.iter_mut().find(|s| s.id == screen_id) {
                    if let Some(a) = screen.anchors.iter_mut().find(|a| a.id == anchor.id) {
                        a.required = required;
                        view_state.screens_dirty = true;
                    }
                }
            }
        }

        // Show expected text for text anchors
        if anchor.anchor_type == AnchorType::Text {
            if let Some(ref expected) = anchor.expected_text {
                ui.label(RichText::new(format!("\"{}\"", expected))
                    .color(ThemeColors::TEXT_SECONDARY)
                    .italics());
            }
        }

        // Delete button
        if ui.small_button("X").on_hover_text("Delete anchor").clicked() {
            delete_clicked = true;
        }
    });

    if delete_clicked {
        Some(anchor.id.clone())
    } else {
        None
    }
}

/// Render a single anchor item
fn render_anchor_item(ui: &mut egui::Ui, anchor: &ScreenAnchor) {
    ui.horizontal(|ui| {
        let icon = match anchor.anchor_type {
            AnchorType::Visual => "[V]",
            AnchorType::Text => "[T]",
        };

        let required_indicator = if anchor.required { "*" } else { "" };

        ui.label(RichText::new(icon).color(ThemeColors::TEXT_MUTED));
        ui.label(RichText::new(&anchor.id).color(ThemeColors::TEXT_PRIMARY));
        ui.label(RichText::new(required_indicator).color(ThemeColors::ACCENT_WARNING));

        if anchor.anchor_type == AnchorType::Text {
            if let Some(ref expected) = anchor.expected_text {
                ui.label(RichText::new(format!("\"{}\"", expected))
                    .color(ThemeColors::TEXT_SECONDARY)
                    .italics());
            }
        }
    });
}

/// Render the add screen dialog
fn render_add_screen_dialog(
    ui: &mut egui::Ui,
    view_state: &mut ScreensViewState,
    shared_state: &Arc<RwLock<SharedAppState>>,
) {
    egui::Window::new("Add Screen")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ui.ctx(), |ui| {
            ui.horizontal(|ui| {
                ui.label("Name:");
                ui.text_edit_singleline(&mut view_state.new_screen_name);
            });

            ui.add_space(8.0);

            ui.horizontal(|ui| {
                ui.label("Match Mode:");
                egui::ComboBox::from_id_salt("match_mode_combo")
                    .selected_text(match view_state.new_screen_match_mode {
                        ScreenMatchMode::Anchors => "Anchors",
                        ScreenMatchMode::FullScreenshot => "Full Screenshot",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut view_state.new_screen_match_mode,
                            ScreenMatchMode::Anchors,
                            "Anchors",
                        );
                        ui.selectable_value(
                            &mut view_state.new_screen_match_mode,
                            ScreenMatchMode::FullScreenshot,
                            "Full Screenshot",
                        );
                    });
            });

            ui.add_space(16.0);

            ui.horizontal(|ui| {
                if ui.button("Cancel").clicked() {
                    view_state.show_add_dialog = false;
                }

                if ui.button("Create").clicked() && !view_state.new_screen_name.is_empty() {
                    let new_screen = ScreenDefinition {
                        id: format!("screen_{}", std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_millis()),
                        name: view_state.new_screen_name.clone(),
                        parent_id: view_state.new_screen_parent_id.clone(),
                        match_mode: view_state.new_screen_match_mode.clone(),
                        anchors: vec![],
                        full_template: None,
                        match_threshold: 0.8,
                        enabled: true,
                        priority: 10,
                        ocr_zone_overrides: vec![],
                        rules_to_trigger: vec![],
                        show_notification: true,
                    };

                    let mut shared = shared_state.write();
                    let active_id = shared.active_profile_id.clone();
                    if let Some(profile) = shared.profiles.iter_mut()
                        .find(|p| active_id.as_ref() == Some(&p.id))
                    {
                        profile.screens.push(new_screen.clone());
                        view_state.selected_screen_id = Some(new_screen.id);
                        view_state.screens_dirty = true;
                    }

                    view_state.show_add_dialog = false;
                    view_state.new_screen_name.clear();
                }
            });
        });
}

/// Render the delete confirmation dialog
fn render_delete_confirm_dialog(
    ui: &mut egui::Ui,
    view_state: &mut ScreensViewState,
    shared_state: &Arc<RwLock<SharedAppState>>,
) {
    let screen_name = {
        let shared = shared_state.read();
        view_state.selected_screen_id.as_ref()
            .and_then(|id| shared.active_profile())
            .and_then(|p| {
                view_state.selected_screen_id.as_ref()
                    .and_then(|id| p.screens.iter().find(|s| &s.id == id))
            })
            .map(|s| s.name.clone())
            .unwrap_or_else(|| "this screen".to_string())
    };

    egui::Window::new("Confirm Delete")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ui.ctx(), |ui| {
            ui.label(format!("Are you sure you want to delete \"{}\"?", screen_name));
            ui.add_space(4.0);
            ui.label(RichText::new("This will also delete all anchors for this screen.")
                .color(ThemeColors::ACCENT_WARNING));
            ui.add_space(4.0);
            ui.label(RichText::new("This action cannot be undone.")
                .color(ThemeColors::ACCENT_ERROR));

            ui.add_space(16.0);

            ui.horizontal(|ui| {
                if ui.button("Cancel").clicked() {
                    view_state.show_delete_confirm = false;
                }

                if ui.add(
                    egui::Button::new(RichText::new("Delete").color(egui::Color32::WHITE))
                        .fill(ThemeColors::ACCENT_ERROR)
                ).clicked() {
                    if let Some(screen_id) = view_state.selected_screen_id.take() {
                        let mut shared = shared_state.write();
                        let active_id = shared.active_profile_id.clone();
                        if let Some(profile) = shared.profiles.iter_mut()
                            .find(|p| active_id.as_ref() == Some(&p.id))
                        {
                            profile.screens.retain(|s| s.id != screen_id);
                            view_state.screens_dirty = true;
                        }
                    }
                    view_state.show_delete_confirm = false;
                }
            });
        });
}

/// Render the text anchor confirmation dialog
fn render_text_anchor_dialog(
    ui: &mut egui::Ui,
    view_state: &mut ScreensViewState,
    shared_state: &Arc<RwLock<SharedAppState>>,
) {
    let Some((screen_id, detected_text, bounds)) = view_state.pending_text_for_anchor.clone() else {
        return;
    };

    // Initialize editing text if empty
    if view_state.editing_text_anchor_text.is_empty() {
        view_state.editing_text_anchor_text = detected_text.clone();
    }

    egui::Window::new("Confirm Text Anchor")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ui.ctx(), |ui| {
            ui.label(RichText::new("Text detected in selected region:")
                .color(ThemeColors::TEXT_SECONDARY));
            ui.add_space(4.0);

            // Show detected text for reference
            ui.label(RichText::new(format!("Detected: \"{}\"", detected_text))
                .color(ThemeColors::TEXT_MUTED)
                .italics());

            ui.add_space(8.0);

            ui.label(RichText::new("Expected text (editable):")
                .color(ThemeColors::TEXT_SECONDARY));
            ui.text_edit_singleline(&mut view_state.editing_text_anchor_text);

            ui.add_space(16.0);

            ui.horizontal(|ui| {
                if ui.button("Cancel").clicked() {
                    view_state.pending_text_for_anchor = None;
                    view_state.editing_text_anchor_text.clear();
                }

                let can_create = !view_state.editing_text_anchor_text.is_empty();
                ui.add_enabled_ui(can_create, |ui| {
                    if ui.button("Create Anchor").clicked() {
                        // Create the text anchor
                        let new_anchor = ScreenAnchor {
                            id: format!("anchor_{}", std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_millis()),
                            anchor_type: AnchorType::Text,
                            bounds,
                            template_data: None,
                            expected_text: Some(view_state.editing_text_anchor_text.clone()),
                            text_similarity: 0.8,
                            required: true,
                        };

                        // Add to screen
                        let mut shared = shared_state.write();
                        let active_id = shared.active_profile_id.clone();
                        if let Some(profile) = shared.profiles.iter_mut()
                            .find(|p| active_id.as_ref() == Some(&p.id))
                        {
                            if let Some(screen) = profile.screens.iter_mut()
                                .find(|s| s.id == screen_id)
                            {
                                screen.anchors.push(new_anchor);
                                view_state.screens_dirty = true;
                            }
                        }

                        view_state.pending_text_for_anchor = None;
                        view_state.editing_text_anchor_text.clear();
                    }
                });
            });
        });
}
