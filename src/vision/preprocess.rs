//! Image preprocessing for OCR models
//!
//! Handles image resizing, normalization, and tensor conversion for PaddleOCR models.

use ndarray::{Array3, Array4, ArrayView3};

/// Preprocessing configuration
#[derive(Debug, Clone)]
pub struct PreprocessConfig {
    /// Target height for detection model (typically 640 or 960)
    pub det_target_size: u32,
    /// Target height for recognition model (typically 32 or 48)
    pub rec_target_height: u32,
    /// Maximum width for recognition (typically 320)
    pub rec_max_width: u32,
    /// Mean values for normalization [R, G, B]
    pub mean: [f32; 3],
    /// Std values for normalization [R, G, B]
    pub std: [f32; 3],
}

impl Default for PreprocessConfig {
    fn default() -> Self {
        Self {
            det_target_size: 960,  // Larger size for better detection on high-res screens
            rec_target_height: 48,
            rec_max_width: 640,    // Allow wider text regions (was 320)
            // PaddleOCR uses simple 0-1 normalization (not ImageNet style)
            // The model expects: (pixel / 255.0 - 0.5) / 0.5 = pixel / 127.5 - 1.0
            // This maps [0, 255] -> [-1, 1]
            mean: [0.5, 0.5, 0.5],
            std: [0.5, 0.5, 0.5],
        }
    }
}

/// Convert RGBA image data to RGB f32 array
/// Note: CapturedFrame stores data as RGBA (converted from Windows BGRA at capture time)
pub fn rgba_to_rgb_f32(data: &[u8], width: u32, height: u32) -> Array3<f32> {
    let mut rgb = Array3::<f32>::zeros((height as usize, width as usize, 3));

    for y in 0..height as usize {
        for x in 0..width as usize {
            let idx = (y * width as usize + x) * 4;
            if idx + 2 < data.len() {
                // RGBA to RGB, normalize to 0-1
                rgb[[y, x, 0]] = data[idx] as f32 / 255.0;     // R
                rgb[[y, x, 1]] = data[idx + 1] as f32 / 255.0; // G
                rgb[[y, x, 2]] = data[idx + 2] as f32 / 255.0; // B
            }
        }
    }

    rgb
}

/// Normalize image with mean and std
pub fn normalize(image: &Array3<f32>, mean: &[f32; 3], std: &[f32; 3]) -> Array3<f32> {
    let (h, w, _) = image.dim();
    let mut normalized = Array3::<f32>::zeros((h, w, 3));

    for y in 0..h {
        for x in 0..w {
            for c in 0..3 {
                normalized[[y, x, c]] = (image[[y, x, c]] - mean[c]) / std[c];
            }
        }
    }

    normalized
}

/// Convert HWC image to NCHW tensor (batch size 1)
pub fn hwc_to_nchw(image: &Array3<f32>) -> Array4<f32> {
    let (h, w, c) = image.dim();
    let mut tensor = Array4::<f32>::zeros((1, c, h, w));

    for y in 0..h {
        for x in 0..w {
            for ch in 0..c {
                tensor[[0, ch, y, x]] = image[[y, x, ch]];
            }
        }
    }

    tensor
}

/// Resize image to target size while maintaining aspect ratio
/// Returns (resized_image, scale_factor)
pub fn resize_for_detection(
    image: &Array3<f32>,
    target_size: u32,
) -> (Array3<f32>, f32) {
    let (h, w, c) = image.dim();
    let h = h as f32;
    let w = w as f32;

    // Calculate scale to fit within target_size
    let scale = target_size as f32 / h.max(w);
    let new_h = (h * scale) as usize;
    let new_w = (w * scale) as usize;

    // Pad to make dimensions divisible by 32 (required by model)
    let padded_h = ((new_h + 31) / 32) * 32;
    let padded_w = ((new_w + 31) / 32) * 32;

    // Bilinear interpolation resize
    let mut resized = Array3::<f32>::zeros((padded_h, padded_w, c));

    for y in 0..new_h {
        for x in 0..new_w {
            let src_y = (y as f32 / scale).min(h - 1.0);
            let src_x = (x as f32 / scale).min(w - 1.0);

            // Bilinear interpolation
            let y0 = src_y.floor() as usize;
            let y1 = (y0 + 1).min(h as usize - 1);
            let x0 = src_x.floor() as usize;
            let x1 = (x0 + 1).min(w as usize - 1);

            let fy = src_y - y0 as f32;
            let fx = src_x - x0 as f32;

            for ch in 0..c {
                let v00 = image[[y0, x0, ch]];
                let v01 = image[[y0, x1, ch]];
                let v10 = image[[y1, x0, ch]];
                let v11 = image[[y1, x1, ch]];

                let v0 = v00 * (1.0 - fx) + v01 * fx;
                let v1 = v10 * (1.0 - fx) + v11 * fx;
                resized[[y, x, ch]] = v0 * (1.0 - fy) + v1 * fy;
            }
        }
    }

    (resized, scale)
}

/// Resize image for recognition model (fixed height, variable width)
pub fn resize_for_recognition(
    image: &Array3<f32>,
    target_height: u32,
    max_width: u32,
) -> Array3<f32> {
    let (h, w, c) = image.dim();
    let h = h as f32;
    let w = w as f32;

    // Scale to target height
    let scale = target_height as f32 / h;
    let new_w = ((w * scale) as u32).min(max_width) as usize;
    let new_h = target_height as usize;

    let mut resized = Array3::<f32>::zeros((new_h, new_w, c));
    let h = h as usize;

    for y in 0..new_h {
        for x in 0..new_w {
            let src_y = (y as f32 / scale).min(h as f32 - 1.0);
            let src_x = (x as f32 / scale).min(w as f32 - 1.0);

            // Bilinear interpolation
            let y0 = src_y.floor() as usize;
            let y1 = (y0 + 1).min(h - 1);
            let x0 = src_x.floor() as usize;
            let x1 = (x0 + 1).min(w as usize - 1);

            let fy = src_y - y0 as f32;
            let fx = src_x - x0 as f32;

            for ch in 0..c {
                let v00 = image[[y0, x0, ch]];
                let v01 = image[[y0, x1, ch]];
                let v10 = image[[y1, x0, ch]];
                let v11 = image[[y1, x1, ch]];

                let v0 = v00 * (1.0 - fx) + v01 * fx;
                let v1 = v10 * (1.0 - fx) + v11 * fx;
                resized[[y, x, ch]] = v0 * (1.0 - fy) + v1 * fy;
            }
        }
    }

    resized
}

/// Crop a region from an image given a polygon (4 points)
pub fn crop_polygon(
    image: &Array3<f32>,
    polygon: &[(f32, f32); 4],
) -> Array3<f32> {
    // Calculate bounding box
    let min_x = polygon.iter().map(|p| p.0).fold(f32::INFINITY, f32::min);
    let min_y = polygon.iter().map(|p| p.1).fold(f32::INFINITY, f32::min);
    let max_x = polygon.iter().map(|p| p.0).fold(f32::NEG_INFINITY, f32::max);
    let max_y = polygon.iter().map(|p| p.1).fold(f32::NEG_INFINITY, f32::max);

    let (img_h, img_w, c) = image.dim();

    let x1 = (min_x as usize).max(0).min(img_w - 1);
    let y1 = (min_y as usize).max(0).min(img_h - 1);
    let x2 = (max_x as usize).max(0).min(img_w);
    let y2 = (max_y as usize).max(0).min(img_h);

    let crop_w = x2 - x1;
    let crop_h = y2 - y1;

    if crop_w == 0 || crop_h == 0 {
        return Array3::<f32>::zeros((1, 1, c));
    }

    let mut cropped = Array3::<f32>::zeros((crop_h, crop_w, c));

    for y in 0..crop_h {
        for x in 0..crop_w {
            for ch in 0..c {
                cropped[[y, x, ch]] = image[[y1 + y, x1 + x, ch]];
            }
        }
    }

    cropped
}

/// Full preprocessing pipeline for detection
pub fn preprocess_for_detection(
    data: &[u8],
    width: u32,
    height: u32,
    config: &PreprocessConfig,
) -> (Array4<f32>, f32) {
    // 1. Convert RGBA to RGB f32
    let rgb = rgba_to_rgb_f32(data, width, height);

    // 2. Resize while maintaining aspect ratio
    let (resized, scale) = resize_for_detection(&rgb, config.det_target_size);

    // 3. Normalize
    let normalized = normalize(&resized, &config.mean, &config.std);

    // 4. Convert to NCHW tensor
    let tensor = hwc_to_nchw(&normalized);

    (tensor, scale)
}

/// Full preprocessing pipeline for recognition
pub fn preprocess_for_recognition(
    image: &Array3<f32>,
    config: &PreprocessConfig,
) -> Array4<f32> {
    // 1. Resize to fixed height
    let resized = resize_for_recognition(image, config.rec_target_height, config.rec_max_width);

    // 2. Normalize
    let normalized = normalize(&resized, &config.mean, &config.std);

    // 3. Convert to NCHW tensor
    hwc_to_nchw(&normalized)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rgba_to_rgb() {
        // Create a 2x2 RGBA image
        let rgba = vec![
            255, 0, 0, 255,     // Red pixel (RGBA)
            0, 255, 0, 255,     // Green pixel
            0, 0, 255, 255,     // Blue pixel
            128, 128, 128, 255, // Gray pixel
        ];

        let rgb = rgba_to_rgb_f32(&rgba, 2, 2);

        // Check red pixel at (0,0)
        assert!((rgb[[0, 0, 0]] - 1.0).abs() < 0.01); // R = 1.0
        assert!(rgb[[0, 0, 1]].abs() < 0.01);         // G = 0.0
        assert!(rgb[[0, 0, 2]].abs() < 0.01);         // B = 0.0

        // Check green pixel at (0,1)
        assert!(rgb[[0, 1, 0]].abs() < 0.01);         // R = 0.0
        assert!((rgb[[0, 1, 1]] - 1.0).abs() < 0.01); // G = 1.0
        assert!(rgb[[0, 1, 2]].abs() < 0.01);         // B = 0.0
    }

    #[test]
    fn test_hwc_to_nchw() {
        let hwc = Array3::<f32>::from_shape_fn((10, 20, 3), |(h, w, c)| {
            (h * 100 + w * 10 + c) as f32
        });

        let nchw = hwc_to_nchw(&hwc);

        assert_eq!(nchw.dim(), (1, 3, 10, 20));
        // Check a sample value
        assert_eq!(nchw[[0, 1, 5, 10]], hwc[[5, 10, 1]]);
    }

    #[test]
    fn test_normalize() {
        let image = Array3::<f32>::from_elem((2, 2, 3), 0.5);
        let mean = [0.485, 0.456, 0.406];
        let std = [0.229, 0.224, 0.225];

        let normalized = normalize(&image, &mean, &std);

        // Check that normalization was applied
        assert!((normalized[[0, 0, 0]] - (0.5 - 0.485) / 0.229).abs() < 0.01);
    }
}
