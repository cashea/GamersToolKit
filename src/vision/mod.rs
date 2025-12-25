//! Vision/OCR Layer
//!
//! Performs text extraction and visual element detection on captured frames.
//! Uses PaddleOCR via ONNX Runtime for text recognition.

pub mod ocr;
pub mod detection;

use anyhow::Result;
use crate::capture::frame::CapturedFrame;

/// Detected text region from OCR
#[derive(Debug, Clone)]
pub struct TextRegion {
    /// Detected text content
    pub text: String,
    /// Bounding box (x, y, width, height)
    pub bounds: (u32, u32, u32, u32),
    /// Confidence score (0.0 - 1.0)
    pub confidence: f32,
}

/// Visual element detected in frame
#[derive(Debug, Clone)]
pub struct VisualElement {
    /// Element identifier from game profile
    pub id: String,
    /// Bounding box (x, y, width, height)
    pub bounds: (u32, u32, u32, u32),
    /// Match confidence (0.0 - 1.0)
    pub confidence: f32,
}

/// Vision processing pipeline
pub struct VisionPipeline {
    // TODO: OCR model
    // TODO: Template matcher
}

impl VisionPipeline {
    /// Create a new vision pipeline
    pub fn new() -> Result<Self> {
        Ok(Self {})
    }

    /// Process a captured frame and extract text/visual elements
    pub fn process(&self, _frame: &CapturedFrame) -> Result<VisionResult> {
        // TODO: Run OCR and detection
        Ok(VisionResult {
            text_regions: vec![],
            visual_elements: vec![],
        })
    }
}

/// Result of vision processing on a frame
#[derive(Debug)]
pub struct VisionResult {
    /// Detected text regions
    pub text_regions: Vec<TextRegion>,
    /// Detected visual elements
    pub visual_elements: Vec<VisualElement>,
}
