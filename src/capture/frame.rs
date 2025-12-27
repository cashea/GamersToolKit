//! Frame data structures for captured screen content

use std::time::Instant;

/// A captured frame from the screen
#[derive(Debug, Clone)]
pub struct CapturedFrame {
    /// Raw RGBA pixel data
    pub data: Vec<u8>,
    /// Frame width in pixels
    pub width: u32,
    /// Frame height in pixels
    pub height: u32,
    /// Timestamp when frame was captured
    pub timestamp: Instant,
}

impl CapturedFrame {
    /// Create a new captured frame with RGBA data
    pub fn new(data: Vec<u8>, width: u32, height: u32) -> Self {
        Self {
            data,
            width,
            height,
            timestamp: Instant::now(),
        }
    }

    /// Create a new captured frame from BGRA data (Windows native format)
    /// Converts BGRA to RGBA in-place for compatibility with image processing
    pub fn new_bgra(mut data: Vec<u8>, width: u32, height: u32) -> Self {
        // Convert BGRA to RGBA by swapping B and R channels
        for chunk in data.chunks_exact_mut(4) {
            chunk.swap(0, 2); // Swap B and R
        }
        Self {
            data,
            width,
            height,
            timestamp: Instant::now(),
        }
    }

    /// Get frame dimensions as (width, height)
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Get the total number of pixels
    pub fn pixel_count(&self) -> u32 {
        self.width * self.height
    }

    /// Get bytes per row (stride)
    pub fn stride(&self) -> u32 {
        self.width * 4
    }

    /// Extract a region of interest from the frame
    /// Returns None if the region is out of bounds
    pub fn extract_region(&self, x: u32, y: u32, w: u32, h: u32) -> Option<CapturedFrame> {
        if x + w > self.width || y + h > self.height {
            return None;
        }

        let mut region_data = Vec::with_capacity((w * h * 4) as usize);
        let stride = self.stride() as usize;

        for row in 0..h {
            let src_start = ((y + row) as usize * stride) + (x as usize * 4);
            let src_end = src_start + (w as usize * 4);
            region_data.extend_from_slice(&self.data[src_start..src_end]);
        }

        Some(CapturedFrame {
            data: region_data,
            width: w,
            height: h,
            timestamp: self.timestamp,
        })
    }

    /// Convert to an image::RgbaImage for further processing
    pub fn to_rgba_image(&self) -> Option<image::RgbaImage> {
        image::RgbaImage::from_raw(self.width, self.height, self.data.clone())
    }

    /// Get a grayscale version of the frame for OCR
    pub fn to_grayscale(&self) -> Vec<u8> {
        self.data
            .chunks_exact(4)
            .map(|rgba| {
                // Luminance formula: 0.299*R + 0.587*G + 0.114*B
                let r = rgba[0] as f32;
                let g = rgba[1] as f32;
                let b = rgba[2] as f32;
                (0.299 * r + 0.587 * g + 0.114 * b) as u8
            })
            .collect()
    }

    /// Get the age of this frame
    pub fn age(&self) -> std::time::Duration {
        self.timestamp.elapsed()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Create a simple 2x2 RGBA test image
    fn create_test_frame() -> CapturedFrame {
        // 2x2 image: Red, Green, Blue, White pixels
        let data = vec![
            255, 0, 0, 255,     // Red (R,G,B,A)
            0, 255, 0, 255,     // Green
            0, 0, 255, 255,     // Blue
            255, 255, 255, 255, // White
        ];
        CapturedFrame::new(data, 2, 2)
    }

    #[test]
    fn test_new_frame() {
        let frame = create_test_frame();
        assert_eq!(frame.width, 2);
        assert_eq!(frame.height, 2);
        assert_eq!(frame.data.len(), 16); // 2x2x4 bytes
    }

    #[test]
    fn test_dimensions() {
        let frame = create_test_frame();
        assert_eq!(frame.dimensions(), (2, 2));
    }

    #[test]
    fn test_pixel_count() {
        let frame = create_test_frame();
        assert_eq!(frame.pixel_count(), 4);
    }

    #[test]
    fn test_stride() {
        let frame = create_test_frame();
        assert_eq!(frame.stride(), 8); // 2 pixels * 4 bytes
    }

    #[test]
    fn test_bgra_to_rgba_conversion() {
        // BGRA data: Blue channel first
        let bgra_data = vec![
            0, 0, 255, 255,     // Blue in BGRA = Red in RGBA
            0, 255, 0, 255,     // Green stays green
            255, 0, 0, 255,     // Red in BGRA = Blue in RGBA
            255, 255, 255, 255, // White stays white
        ];

        let frame = CapturedFrame::new_bgra(bgra_data, 2, 2);

        // After conversion, first pixel should be Red (R=255, G=0, B=0)
        assert_eq!(frame.data[0], 255); // R
        assert_eq!(frame.data[1], 0);   // G
        assert_eq!(frame.data[2], 0);   // B
        assert_eq!(frame.data[3], 255); // A

        // Third pixel should be Blue (R=0, G=0, B=255)
        assert_eq!(frame.data[8], 0);   // R
        assert_eq!(frame.data[9], 0);   // G
        assert_eq!(frame.data[10], 255); // B
    }

    #[test]
    fn test_extract_region_valid() {
        // Create a 4x4 frame
        let mut data = Vec::with_capacity(64);
        for i in 0..16 {
            data.extend_from_slice(&[i as u8, i as u8, i as u8, 255]);
        }
        let frame = CapturedFrame::new(data, 4, 4);

        // Extract 2x2 region from (1,1)
        let region = frame.extract_region(1, 1, 2, 2);
        assert!(region.is_some());

        let region = region.unwrap();
        assert_eq!(region.width, 2);
        assert_eq!(region.height, 2);
        assert_eq!(region.data.len(), 16); // 2x2x4 bytes
    }

    #[test]
    fn test_extract_region_out_of_bounds() {
        let frame = create_test_frame();

        // Try to extract region that goes out of bounds
        let region = frame.extract_region(1, 1, 2, 2);
        assert!(region.is_none());
    }

    #[test]
    fn test_extract_region_at_edge() {
        let frame = create_test_frame();

        // Extract single pixel at bottom-right
        let region = frame.extract_region(1, 1, 1, 1);
        assert!(region.is_some());

        let region = region.unwrap();
        assert_eq!(region.width, 1);
        assert_eq!(region.height, 1);
        // Should be the white pixel
        assert_eq!(region.data, vec![255, 255, 255, 255]);
    }

    #[test]
    fn test_to_grayscale() {
        let frame = create_test_frame();
        let gray = frame.to_grayscale();

        assert_eq!(gray.len(), 4); // 4 pixels

        // Red pixel: 0.299 * 255 = ~76
        assert!((gray[0] as i32 - 76).abs() < 2);

        // Green pixel: 0.587 * 255 = ~150
        assert!((gray[1] as i32 - 150).abs() < 2);

        // Blue pixel: 0.114 * 255 = ~29
        assert!((gray[2] as i32 - 29).abs() < 2);

        // White pixel: 0.299*255 + 0.587*255 + 0.114*255 = 255
        assert_eq!(gray[3], 255);
    }

    #[test]
    fn test_to_rgba_image() {
        let frame = create_test_frame();
        let img = frame.to_rgba_image();

        assert!(img.is_some());
        let img = img.unwrap();
        assert_eq!(img.width(), 2);
        assert_eq!(img.height(), 2);
    }

    #[test]
    fn test_age() {
        let frame = create_test_frame();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let age = frame.age();
        assert!(age.as_millis() >= 10);
    }
}
