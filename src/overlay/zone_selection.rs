//! Zone selection overlay for defining OCR regions
//!
//! Provides interactive rectangle drawing on the overlay
//! for defining screen regions to OCR.

use egui::{Color32, FontId, Key, Pos2, Rect, Rounding, Stroke};

use crate::overlay::ZoneSelectionResult;

/// State for zone selection in overlay
#[derive(Debug, Clone, Default)]
pub struct ZoneSelectionOverlayState {
    /// Current selection start point (screen pixels)
    pub start_point: Option<Pos2>,
    /// Current mouse position (screen pixels)
    pub current_point: Option<Pos2>,
    /// Finalized selection (normalized 0.0-1.0): (x, y, width, height)
    pub completed_selection: Option<(f32, f32, f32, f32)>,
    /// Existing zones to display: (name, normalized bounds)
    pub existing_zones: Vec<(String, (f32, f32, f32, f32))>,
}

/// Render zone selection UI on the overlay
///
/// Returns Some(ZoneSelectionResult) when the user completes or cancels selection.
pub fn render_zone_selection(
    ctx: &egui::Context,
    state: &mut ZoneSelectionOverlayState,
    screen_size: (f32, f32),
) -> Option<ZoneSelectionResult> {
    let mut result: Option<ZoneSelectionResult> = None;

    // Check for ESC key to cancel
    if ctx.input(|i| i.key_pressed(Key::Escape)) {
        return Some(ZoneSelectionResult::Cancelled);
    }

    // Full-screen area for zone selection
    egui::Area::new(egui::Id::new("zone_selection_overlay"))
        .fixed_pos(Pos2::ZERO)
        .show(ctx, |ui| {
            let screen_rect = Rect::from_min_size(Pos2::ZERO, egui::vec2(screen_size.0, screen_size.1));

            // Allocate the full screen for interaction
            let response = ui.allocate_rect(screen_rect, egui::Sense::click_and_drag());

            let painter = ui.painter();

            // Draw semi-transparent dark overlay
            painter.rect_filled(
                screen_rect,
                Rounding::ZERO,
                Color32::from_rgba_unmultiplied(0, 0, 0, 120),
            );

            // Draw existing zones with green dashed rectangles
            for (name, bounds) in &state.existing_zones {
                let zone_rect = normalized_to_screen(*bounds, screen_size);

                // Draw filled background (semi-transparent)
                painter.rect_filled(
                    zone_rect,
                    Rounding::same(2.0),
                    Color32::from_rgba_unmultiplied(0, 150, 0, 40),
                );

                // Draw border
                painter.rect_stroke(
                    zone_rect,
                    Rounding::same(2.0),
                    Stroke::new(2.0, Color32::from_rgb(0, 200, 0)),
                );

                // Draw zone name label
                let label_pos = zone_rect.left_top() + egui::vec2(4.0, -18.0);
                painter.text(
                    label_pos,
                    egui::Align2::LEFT_TOP,
                    name,
                    FontId::proportional(14.0),
                    Color32::from_rgb(0, 255, 0),
                );
            }

            // Handle mouse interaction
            let pointer_pos = response.interact_pointer_pos();

            if response.drag_started() {
                if let Some(pos) = pointer_pos {
                    state.start_point = Some(pos);
                    state.current_point = Some(pos);
                }
            }

            if response.dragged() {
                if let Some(pos) = pointer_pos {
                    state.current_point = Some(pos);
                }
            }

            if response.drag_stopped() {
                if let (Some(start), Some(end)) = (state.start_point, state.current_point) {
                    // Calculate normalized bounds
                    let bounds = screen_to_normalized(start, end, screen_size);

                    // Only accept if the zone has a minimum size (at least 10x10 pixels)
                    let width_px = bounds.2 * screen_size.0;
                    let height_px = bounds.3 * screen_size.1;

                    if width_px >= 10.0 && height_px >= 10.0 {
                        result = Some(ZoneSelectionResult::Completed { bounds });
                    }
                }

                // Clear selection points
                state.start_point = None;
                state.current_point = None;
            }

            // Draw current selection rectangle
            if let (Some(start), Some(current)) = (state.start_point, state.current_point) {
                let selection_rect = Rect::from_two_pos(start, current);

                // Draw selection with blue color
                painter.rect_filled(
                    selection_rect,
                    Rounding::same(2.0),
                    Color32::from_rgba_unmultiplied(0, 100, 255, 60),
                );
                painter.rect_stroke(
                    selection_rect,
                    Rounding::same(2.0),
                    Stroke::new(2.0, Color32::from_rgb(0, 150, 255)),
                );

                // Show size info
                let width = selection_rect.width().abs();
                let height = selection_rect.height().abs();
                let size_text = format!("{:.0} x {:.0}", width, height);

                let text_pos = selection_rect.center();
                painter.text(
                    text_pos,
                    egui::Align2::CENTER_CENTER,
                    &size_text,
                    FontId::proportional(16.0),
                    Color32::WHITE,
                );
            }

            // Draw instructions at the top
            let instructions = "Click and drag to select a region for OCR. Press ESC to cancel.";
            let instruction_pos = Pos2::new(screen_size.0 / 2.0, 30.0);

            // Draw background for text
            let text_galley = painter.layout_no_wrap(
                instructions.to_string(),
                FontId::proportional(16.0),
                Color32::WHITE,
            );
            let text_rect = Rect::from_center_size(
                instruction_pos,
                text_galley.size() + egui::vec2(20.0, 10.0),
            );
            painter.rect_filled(
                text_rect,
                Rounding::same(4.0),
                Color32::from_rgba_unmultiplied(0, 0, 0, 200),
            );

            painter.text(
                instruction_pos,
                egui::Align2::CENTER_CENTER,
                instructions,
                FontId::proportional(16.0),
                Color32::WHITE,
            );
        });

    result
}

/// Convert two screen positions to normalized bounds (0.0-1.0)
///
/// Returns (x, y, width, height) normalized to screen size.
pub fn screen_to_normalized(
    start: Pos2,
    end: Pos2,
    screen_size: (f32, f32),
) -> (f32, f32, f32, f32) {
    let min_x = start.x.min(end.x);
    let min_y = start.y.min(end.y);
    let max_x = start.x.max(end.x);
    let max_y = start.y.max(end.y);

    let x = min_x / screen_size.0;
    let y = min_y / screen_size.1;
    let width = (max_x - min_x) / screen_size.0;
    let height = (max_y - min_y) / screen_size.1;

    (x.clamp(0.0, 1.0), y.clamp(0.0, 1.0), width.clamp(0.0, 1.0), height.clamp(0.0, 1.0))
}

/// Convert normalized bounds to screen coordinates
pub fn normalized_to_screen(
    bounds: (f32, f32, f32, f32),
    screen_size: (f32, f32),
) -> Rect {
    let (x, y, width, height) = bounds;
    Rect::from_min_size(
        Pos2::new(x * screen_size.0, y * screen_size.1),
        egui::vec2(width * screen_size.0, height * screen_size.1),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_screen_to_normalized() {
        let start = Pos2::new(100.0, 200.0);
        let end = Pos2::new(300.0, 400.0);
        let screen_size = (1000.0, 1000.0);

        let (x, y, w, h) = screen_to_normalized(start, end, screen_size);

        assert!((x - 0.1).abs() < 0.001);
        assert!((y - 0.2).abs() < 0.001);
        assert!((w - 0.2).abs() < 0.001);
        assert!((h - 0.2).abs() < 0.001);
    }

    #[test]
    fn test_screen_to_normalized_reversed() {
        // Test with start > end (user dragged backwards)
        let start = Pos2::new(300.0, 400.0);
        let end = Pos2::new(100.0, 200.0);
        let screen_size = (1000.0, 1000.0);

        let (x, y, w, h) = screen_to_normalized(start, end, screen_size);

        // Should still produce valid normalized bounds
        assert!((x - 0.1).abs() < 0.001);
        assert!((y - 0.2).abs() < 0.001);
        assert!((w - 0.2).abs() < 0.001);
        assert!((h - 0.2).abs() < 0.001);
    }

    #[test]
    fn test_normalized_to_screen() {
        let bounds = (0.1, 0.2, 0.2, 0.2);
        let screen_size = (1000.0, 1000.0);

        let rect = normalized_to_screen(bounds, screen_size);

        assert!((rect.min.x - 100.0).abs() < 0.001);
        assert!((rect.min.y - 200.0).abs() < 0.001);
        assert!((rect.width() - 200.0).abs() < 0.001);
        assert!((rect.height() - 200.0).abs() < 0.001);
    }
}
