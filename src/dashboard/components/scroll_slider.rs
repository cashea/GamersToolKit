//! Scroll-enabled slider component
//!
//! A slider that can be adjusted using the mouse wheel when hovered.

use egui::{Response, Slider, Ui};
use std::ops::RangeInclusive;

/// Add a scroll-enabled slider and return the response.
///
/// When the slider is hovered, the mouse wheel can be used to adjust the value.
/// The step size for scrolling is either provided explicitly or calculated
/// automatically as 1/20th of the range.
///
/// # Arguments
/// * `ui` - The egui UI context
/// * `value` - Mutable reference to the value to control
/// * `range` - The valid range for the value
/// * `step` - Optional step size for both drag and scroll (auto-calculated if None)
/// * `suffix` - Optional suffix to display after the value (e.g., " px", " fps")
/// * `fixed_decimals` - Optional number of decimal places to display
pub fn add_scroll_slider<Num: egui::emath::Numeric>(
    ui: &mut Ui,
    value: &mut Num,
    range: RangeInclusive<Num>,
    step: Option<f64>,
    suffix: Option<&str>,
    fixed_decimals: Option<usize>,
) -> Response {
    let range_f64 = (*range.start()).to_f64()..=(*range.end()).to_f64();
    let range_span = range_f64.end() - range_f64.start();

    // Calculate scroll step: use provided step, or 1/20th of range (min 1 for integers)
    let scroll_step = step.unwrap_or_else(|| {
        let auto_step = range_span / 20.0;
        // For integer types, ensure minimum step of 1
        if std::any::TypeId::of::<Num>() == std::any::TypeId::of::<i32>()
            || std::any::TypeId::of::<Num>() == std::any::TypeId::of::<u32>()
            || std::any::TypeId::of::<Num>() == std::any::TypeId::of::<i64>()
            || std::any::TypeId::of::<Num>() == std::any::TypeId::of::<u64>()
            || std::any::TypeId::of::<Num>() == std::any::TypeId::of::<usize>()
            || std::any::TypeId::of::<Num>() == std::any::TypeId::of::<isize>()
        {
            auto_step.max(1.0)
        } else {
            auto_step
        }
    });

    // Build the slider
    let mut slider = Slider::new(value, range);
    if let Some(s) = step {
        slider = slider.step_by(s);
    }
    if let Some(s) = suffix {
        slider = slider.suffix(s);
    }
    if let Some(d) = fixed_decimals {
        slider = slider.fixed_decimals(d);
    }

    let response = ui.add(slider);

    // Handle scroll wheel when hovered
    if response.hovered() {
        let scroll_delta = ui.input(|i| i.raw_scroll_delta.y);
        if scroll_delta != 0.0 {
            let current = (*value).to_f64();
            // Scroll up (positive delta) increases value, scroll down decreases
            let direction = if scroll_delta > 0.0 { 1.0 } else { -1.0 };
            let new_value = (current + direction * scroll_step).clamp(*range_f64.start(), *range_f64.end());
            *value = Num::from_f64(new_value);
            // Request repaint to update UI
            ui.ctx().request_repaint();
        }
    }

    response
}
