//! Image preprocessing filters for OCR optimization
//!
//! Provides optional image enhancements to improve OCR accuracy,
//! especially for game text with stylized fonts or low contrast.

use crate::config::OcrPreprocessing;
use tracing::debug;

/// Result of preprocessing that includes potentially resized dimensions
pub struct PreprocessResult {
    /// Processed image data
    pub data: Vec<u8>,
    /// New width (may differ from original if scaled)
    pub width: u32,
    /// New height (may differ from original if scaled)
    pub height: u32,
}

/// Apply preprocessing filters to RGBA image data based on settings
/// Returns a new Vec with the processed image data (original dimensions)
pub fn apply_preprocessing(data: &[u8], width: u32, height: u32, settings: &OcrPreprocessing) -> Vec<u8> {
    let result = apply_preprocessing_with_scale(data, width, height, settings);
    result.data
}

/// Apply preprocessing filters to RGBA image data based on settings
/// Returns PreprocessResult with potentially scaled dimensions
pub fn apply_preprocessing_with_scale(data: &[u8], width: u32, height: u32, settings: &OcrPreprocessing) -> PreprocessResult {
    if !settings.enabled {
        debug!("OCR preprocessing disabled");
        return PreprocessResult {
            data: data.to_vec(),
            width,
            height,
        };
    }

    debug!(
        "OCR preprocessing enabled: grayscale={}, invert={}, contrast={}, sharpen={}, scale={}",
        settings.grayscale, settings.invert, settings.contrast, settings.sharpen, settings.scale
    );

    // Apply upscaling first if requested (before other filters for better quality)
    let (mut result, new_width, new_height) = if settings.scale > 1 {
        let scaled = apply_upscale(data, width, height, settings.scale);
        let new_w = width * settings.scale;
        let new_h = height * settings.scale;
        (scaled, new_w, new_h)
    } else {
        (data.to_vec(), width, height)
    };

    // Apply contrast enhancement (works on color)
    if (settings.contrast - 1.0).abs() > 0.01 {
        apply_contrast(&mut result, settings.contrast);
    }

    // Apply sharpening (works on color)
    if settings.sharpen > 0.01 {
        result = apply_sharpen(&result, new_width, new_height, settings.sharpen);
    }

    // Convert to grayscale if requested (after other filters)
    if settings.grayscale {
        apply_grayscale(&mut result);
    }

    // Invert colors last
    if settings.invert {
        apply_invert(&mut result);
    }

    PreprocessResult {
        data: result,
        width: new_width,
        height: new_height,
    }
}

/// Apply contrast enhancement to RGBA data
/// Factor > 1.0 increases contrast, < 1.0 decreases
fn apply_contrast(data: &mut [u8], factor: f32) {
    for chunk in data.chunks_exact_mut(4) {
        for i in 0..3 {
            let val = chunk[i] as f32;
            // Contrast around midpoint (128)
            let adjusted = ((val - 128.0) * factor + 128.0).clamp(0.0, 255.0);
            chunk[i] = adjusted as u8;
        }
        // Alpha channel unchanged
    }
}

/// Convert RGBA to grayscale (keeping RGBA format for compatibility)
fn apply_grayscale(data: &mut [u8]) {
    for chunk in data.chunks_exact_mut(4) {
        // Standard luminance weights
        let gray = (0.299 * chunk[0] as f32 + 0.587 * chunk[1] as f32 + 0.114 * chunk[2] as f32) as u8;
        chunk[0] = gray;
        chunk[1] = gray;
        chunk[2] = gray;
        // Alpha unchanged
    }
}

/// Invert RGB colors (useful for light text on dark backgrounds)
fn apply_invert(data: &mut [u8]) {
    for chunk in data.chunks_exact_mut(4) {
        chunk[0] = 255 - chunk[0];
        chunk[1] = 255 - chunk[1];
        chunk[2] = 255 - chunk[2];
        // Alpha unchanged
    }
}

/// Apply unsharp mask sharpening to RGBA data
/// Strength 0.0 = no sharpening, 1.0 = strong sharpening
fn apply_sharpen(data: &[u8], width: u32, height: u32, strength: f32) -> Vec<u8> {
    let w = width as usize;
    let h = height as usize;
    let mut result = data.to_vec();

    // Simple 3x3 sharpen kernel
    // Center weight is 1 + 4*strength, neighbors are -strength
    let center_weight = 1.0 + 4.0 * strength;
    let neighbor_weight = -strength;

    // Process each pixel (skip edges for simplicity)
    for y in 1..(h - 1) {
        for x in 1..(w - 1) {
            let idx = (y * w + x) * 4;

            for c in 0..3 {
                // Get neighboring pixels
                let top = data[((y - 1) * w + x) * 4 + c] as f32;
                let bottom = data[((y + 1) * w + x) * 4 + c] as f32;
                let left = data[(y * w + x - 1) * 4 + c] as f32;
                let right = data[(y * w + x + 1) * 4 + c] as f32;
                let center = data[idx + c] as f32;

                // Apply kernel
                let sharpened = center * center_weight
                    + top * neighbor_weight
                    + bottom * neighbor_weight
                    + left * neighbor_weight
                    + right * neighbor_weight;

                result[idx + c] = sharpened.clamp(0.0, 255.0) as u8;
            }
        }
    }

    result
}

/// Upscale RGBA image using bilinear interpolation
/// Scale factor should be >= 1 (1 = no change, 2 = double size, etc.)
fn apply_upscale(data: &[u8], width: u32, height: u32, scale: u32) -> Vec<u8> {
    if scale <= 1 {
        return data.to_vec();
    }

    let new_width = width * scale;
    let new_height = height * scale;
    let mut result = vec![0u8; (new_width * new_height * 4) as usize];

    let w = width as usize;
    let h = height as usize;
    let nw = new_width as usize;
    let scale_f = scale as f32;

    for ny in 0..new_height as usize {
        for nx in 0..nw {
            // Map back to source coordinates
            let src_x = nx as f32 / scale_f;
            let src_y = ny as f32 / scale_f;

            // Get the four nearest source pixels
            let x0 = (src_x.floor() as usize).min(w - 1);
            let y0 = (src_y.floor() as usize).min(h - 1);
            let x1 = (x0 + 1).min(w - 1);
            let y1 = (y0 + 1).min(h - 1);

            // Calculate interpolation weights
            let x_weight = src_x - src_x.floor();
            let y_weight = src_y - src_y.floor();

            let dst_idx = (ny * nw + nx) * 4;

            // Bilinear interpolation for each channel
            for c in 0..4 {
                let p00 = data[(y0 * w + x0) * 4 + c] as f32;
                let p10 = data[(y0 * w + x1) * 4 + c] as f32;
                let p01 = data[(y1 * w + x0) * 4 + c] as f32;
                let p11 = data[(y1 * w + x1) * 4 + c] as f32;

                // Interpolate horizontally first
                let top = p00 * (1.0 - x_weight) + p10 * x_weight;
                let bottom = p01 * (1.0 - x_weight) + p11 * x_weight;

                // Then vertically
                let value = top * (1.0 - y_weight) + bottom * y_weight;

                result[dst_idx + c] = value.clamp(0.0, 255.0) as u8;
            }
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preprocessing_disabled() {
        let data = vec![100, 150, 200, 255];
        let settings = OcrPreprocessing::default();
        let result = apply_preprocessing(&data, 1, 1, &settings);
        assert_eq!(result, data);
    }

    #[test]
    fn test_contrast_increase() {
        let mut data = vec![100, 128, 200, 255];
        apply_contrast(&mut data, 2.0);
        // 100: (100-128)*2+128 = 72
        // 128: (128-128)*2+128 = 128
        // 200: (200-128)*2+128 = 272 -> clamped to 255
        assert_eq!(data[0], 72);
        assert_eq!(data[1], 128);
        assert_eq!(data[2], 255);
        assert_eq!(data[3], 255); // Alpha unchanged
    }

    #[test]
    fn test_grayscale() {
        let mut data = vec![255, 0, 0, 255]; // Red pixel
        apply_grayscale(&mut data);
        // Gray = 0.299*255 + 0.587*0 + 0.114*0 = 76.245 ≈ 76
        assert_eq!(data[0], 76);
        assert_eq!(data[1], 76);
        assert_eq!(data[2], 76);
    }

    #[test]
    fn test_invert() {
        let mut data = vec![0, 100, 255, 255];
        apply_invert(&mut data);
        assert_eq!(data[0], 255);
        assert_eq!(data[1], 155);
        assert_eq!(data[2], 0);
        assert_eq!(data[3], 255); // Alpha unchanged
    }

    #[test]
    fn test_upscale_2x() {
        // 2x2 image (RGBA)
        let data = vec![
            255, 0, 0, 255,   // Red
            0, 255, 0, 255,   // Green
            0, 0, 255, 255,   // Blue
            255, 255, 0, 255, // Yellow
        ];
        let result = apply_upscale(&data, 2, 2, 2);
        // Should be 4x4 image
        assert_eq!(result.len(), 4 * 4 * 4);
    }

    #[test]
    fn test_upscale_noop() {
        let data = vec![100, 150, 200, 255];
        let result = apply_upscale(&data, 1, 1, 1);
        assert_eq!(result, data);
    }

    #[test]
    fn test_preprocessing_with_scale() {
        // 2x2 image
        let data = vec![
            100, 100, 100, 255,
            100, 100, 100, 255,
            100, 100, 100, 255,
            100, 100, 100, 255,
        ];
        let mut settings = OcrPreprocessing::default();
        settings.enabled = true;
        settings.scale = 2;

        let result = apply_preprocessing_with_scale(&data, 2, 2, &settings);
        assert_eq!(result.width, 4);
        assert_eq!(result.height, 4);
        assert_eq!(result.data.len(), 4 * 4 * 4);
    }
}
