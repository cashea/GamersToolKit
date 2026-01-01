//! Vision/OCR Layer
//!
//! Performs text extraction and visual element detection on captured frames.
//! Supports multiple OCR backends:
//! - Windows OCR API (recommended for game text)
//! - PaddleOCR via ONNX Runtime

pub mod detection;
pub mod models;
pub mod ocr;
pub mod ocr_preprocess;
pub mod preprocess;
pub mod screen_recognition;
pub mod windows_ocr;

pub use ocr_preprocess::{PreprocessResult, apply_preprocessing_with_scale};

use anyhow::Result;
use std::time::Instant;
use tracing::{debug, info};

use crate::capture::frame::CapturedFrame;

pub use models::{ModelManager, ModelType, OnnxSession};
pub use ocr::{OcrEngine, OcrResult};
pub use screen_recognition::{ScreenRecognizer, ScreenRecognitionConfig, ScreenMatch, AnchorMatch, ScreenNode};
pub use windows_ocr::{WindowsOcr, WindowsOcrResult, WindowsOcrLine, WindowsOcrFullResult};

/// OCR backend selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OcrBackend {
    /// Windows built-in OCR (recommended for game text)
    #[default]
    WindowsOcr,
    /// PaddleOCR via ONNX Runtime
    PaddleOcr,
}

/// OCR result granularity
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OcrGranularity {
    /// Individual words with their bounding boxes
    #[default]
    Word,
    /// Full lines with their bounding boxes
    Line,
}

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

/// Configuration for the vision pipeline
#[derive(Debug, Clone)]
pub struct VisionConfig {
    /// OCR backend to use
    pub backend: OcrBackend,
    /// Minimum confidence threshold for text detection (0.0 - 1.0)
    pub detection_threshold: f32,
    /// Minimum confidence threshold for text recognition (0.0 - 1.0)
    pub recognition_threshold: f32,
    /// Whether to use GPU acceleration (PaddleOCR only)
    pub use_gpu: bool,
    /// Maximum image dimension for processing (larger images are scaled down)
    pub max_image_size: u32,
    /// Language for Windows OCR (e.g., "en-US")
    pub ocr_language: String,
}

impl Default for VisionConfig {
    fn default() -> Self {
        Self {
            backend: OcrBackend::WindowsOcr, // Default to Windows OCR for game text
            detection_threshold: 0.5,
            recognition_threshold: 0.5,
            use_gpu: true,
            max_image_size: 1920,
            ocr_language: "en-US".to_string(),
        }
    }
}

/// Vision processing pipeline with multiple OCR backends
pub struct VisionPipeline {
    /// PaddleOCR engine (ONNX-based)
    paddle_ocr: Option<OcrEngine>,
    /// Windows OCR engine
    windows_ocr: Option<WindowsOcr>,
    /// Current configuration
    config: VisionConfig,
    /// Model manager for PaddleOCR
    model_manager: ModelManager,
}

impl VisionPipeline {
    /// Create a new vision pipeline with default configuration
    pub fn new() -> Result<Self> {
        Self::with_config(VisionConfig::default())
    }

    /// Create a new vision pipeline with custom configuration
    pub fn with_config(config: VisionConfig) -> Result<Self> {
        let model_manager = ModelManager::new()?;

        Ok(Self {
            paddle_ocr: None,
            windows_ocr: None,
            config,
            model_manager,
        })
    }

    /// Get the current OCR backend
    pub fn backend(&self) -> OcrBackend {
        self.config.backend
    }

    /// Set the OCR backend
    pub fn set_backend(&mut self, backend: OcrBackend) {
        self.config.backend = backend;
    }

    /// Check if models are ready (for PaddleOCR)
    pub fn are_models_ready(&self) -> bool {
        self.model_manager.are_models_ready()
    }

    /// Get model status (for PaddleOCR)
    pub fn get_model_status(&self) -> Vec<(ModelType, bool, Option<u64>)> {
        self.model_manager.get_model_status()
    }

    /// Initialize the OCR engine for the current backend
    pub fn init_ocr(&mut self) -> Result<()> {
        match self.config.backend {
            OcrBackend::WindowsOcr => self.init_windows_ocr(),
            OcrBackend::PaddleOcr => self.init_paddle_ocr(),
        }
    }

    /// Initialize Windows OCR
    fn init_windows_ocr(&mut self) -> Result<()> {
        if self.windows_ocr.is_some() {
            return Ok(());
        }

        info!("Initializing Windows OCR backend");
        let engine = WindowsOcr::new(&self.config.ocr_language)?;
        self.windows_ocr = Some(engine);
        info!("Windows OCR initialized successfully");
        Ok(())
    }

    /// Initialize PaddleOCR
    fn init_paddle_ocr(&mut self) -> Result<()> {
        if self.paddle_ocr.is_some() {
            return Ok(());
        }

        info!("Initializing PaddleOCR backend");

        // Ensure models are available
        let det_path = self.model_manager.ensure_model(ModelType::Detection)?;
        let rec_path = self.model_manager.ensure_model(ModelType::Recognition)?;

        // Initialize OCR engine
        let ocr_engine = OcrEngine::new(
            det_path.to_str().unwrap(),
            rec_path.to_str().unwrap(),
            self.config.use_gpu,
        )?;

        self.paddle_ocr = Some(ocr_engine);
        info!("PaddleOCR initialized successfully");
        Ok(())
    }

    /// Check if OCR is initialized for the current backend
    pub fn is_ocr_ready(&self) -> bool {
        match self.config.backend {
            OcrBackend::WindowsOcr => self.windows_ocr.is_some(),
            OcrBackend::PaddleOcr => self.paddle_ocr.is_some(),
        }
    }

    /// Process a captured frame and extract text/visual elements
    pub fn process(&mut self, frame: &CapturedFrame) -> Result<VisionResult> {
        self.process_with_granularity(frame, OcrGranularity::Word)
    }

    /// Process a captured frame with specified granularity (word or line level)
    pub fn process_with_granularity(&mut self, frame: &CapturedFrame, granularity: OcrGranularity) -> Result<VisionResult> {
        let start = Instant::now();

        let text_regions = match self.config.backend {
            OcrBackend::WindowsOcr => {
                match granularity {
                    OcrGranularity::Word => self.process_windows_ocr(&frame.data, frame.width, frame.height)?,
                    OcrGranularity::Line => self.process_windows_ocr_lines(&frame.data, frame.width, frame.height)?,
                }
            }
            OcrBackend::PaddleOcr => {
                // PaddleOCR already returns line-level results, so we use the same for both
                self.process_paddle_ocr(&frame.data, frame.width, frame.height)?
            }
        };

        let processing_time = start.elapsed();
        debug!(
            "Vision processing ({:?}, {:?}) complete in {:?}: {} text regions",
            self.config.backend,
            granularity,
            processing_time,
            text_regions.len()
        );

        Ok(VisionResult {
            text_regions,
            visual_elements: vec![],
            processing_time_ms: processing_time.as_millis() as u64,
        })
    }

    /// Process using Windows OCR (word-level)
    fn process_windows_ocr(&self, data: &[u8], width: u32, height: u32) -> Result<Vec<TextRegion>> {
        let Some(ocr) = &self.windows_ocr else {
            return Ok(vec![]);
        };

        let results = ocr.recognize(data, width, height)?;

        Ok(results
            .into_iter()
            .filter(|r| r.confidence >= self.config.recognition_threshold)
            .map(|r| TextRegion {
                text: r.text,
                bounds: r.bounds,
                confidence: r.confidence,
            })
            .collect())
    }

    /// Process using Windows OCR (line-level)
    fn process_windows_ocr_lines(&self, data: &[u8], width: u32, height: u32) -> Result<Vec<TextRegion>> {
        let Some(ocr) = &self.windows_ocr else {
            return Ok(vec![]);
        };

        let full_result = ocr.recognize_full(data, width, height)?;

        Ok(full_result.lines
            .into_iter()
            .map(|line| TextRegion {
                text: line.text,
                bounds: line.bounds,
                confidence: 1.0, // Windows OCR doesn't provide line-level confidence
            })
            .collect())
    }

    /// Process using PaddleOCR
    fn process_paddle_ocr(&mut self, data: &[u8], width: u32, height: u32) -> Result<Vec<TextRegion>> {
        let Some(ocr) = &mut self.paddle_ocr else {
            return Ok(vec![]);
        };

        let results = ocr.recognize(data, width, height)?;

        Ok(results
            .into_iter()
            .filter(|r| r.confidence >= self.config.recognition_threshold)
            .map(|r| TextRegion {
                text: r.text,
                bounds: polygon_to_bounds(&r.polygon),
                confidence: r.confidence,
            })
            .collect())
    }

    /// Process a specific region of a frame
    pub fn process_region(
        &mut self,
        frame: &CapturedFrame,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
    ) -> Result<VisionResult> {
        self.process_region_with_preprocessing(frame, x, y, width, height, None)
    }

    /// Process a specific region of a frame with optional custom preprocessing
    pub fn process_region_with_preprocessing(
        &mut self,
        frame: &CapturedFrame,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        preprocessing: Option<&crate::config::OcrPreprocessing>,
    ) -> Result<VisionResult> {
        // Extract the region from the frame
        let region_data = extract_region(&frame.data, frame.width, frame.height, x, y, width, height);

        // Determine auto-upscale factor for small regions
        // Windows OCR needs ~40+ pixel height for reliable detection
        const MIN_OCR_DIMENSION: u32 = 80;
        let auto_scale = if height < MIN_OCR_DIMENSION || width < MIN_OCR_DIMENSION {
            let height_scale = (MIN_OCR_DIMENSION as f32 / height as f32).ceil() as u32;
            let width_scale = (MIN_OCR_DIMENSION as f32 / width as f32).ceil() as u32;
            height_scale.max(width_scale).max(2).min(4) // Scale 2x-4x for small regions
        } else {
            1
        };

        // Apply preprocessing if provided, with auto-upscaling for small regions
        let (processed_data, proc_width, proc_height) = if let Some(pp) = preprocessing {
            // Merge auto-scale with user's scale setting
            let effective_scale = pp.scale.max(auto_scale);
            if effective_scale != pp.scale {
                info!(
                    "Auto-upscaling small region from {}x{} by {}x (user scale: {})",
                    width, height, effective_scale, pp.scale
                );
                let mut adjusted_pp = pp.clone();
                adjusted_pp.scale = effective_scale;
                let result = apply_preprocessing_with_scale(&region_data, width, height, &adjusted_pp);
                (result.data, result.width, result.height)
            } else {
                let result = apply_preprocessing_with_scale(&region_data, width, height, pp);
                (result.data, result.width, result.height)
            }
        } else if auto_scale > 1 {
            // No preprocessing specified but region is small - apply auto-upscaling
            info!(
                "Auto-upscaling small region from {}x{} by {}x",
                width, height, auto_scale
            );
            let auto_pp = crate::config::OcrPreprocessing {
                enabled: true,
                scale: auto_scale,
                ..Default::default()
            };
            let result = apply_preprocessing_with_scale(&region_data, width, height, &auto_pp);
            (result.data, result.width, result.height)
        } else {
            (region_data, width, height)
        };

        let start = Instant::now();

        let text_regions = match self.config.backend {
            OcrBackend::WindowsOcr => {
                let results = self.process_windows_ocr(&processed_data, proc_width, proc_height)?;
                // Offset bounds by region position (scale back if preprocessing scaled)
                let scale_factor = if proc_width != width { width as f32 / proc_width as f32 } else { 1.0 };
                results
                    .into_iter()
                    .map(|r| TextRegion {
                        text: r.text,
                        bounds: (
                            x + (r.bounds.0 as f32 * scale_factor) as u32,
                            y + (r.bounds.1 as f32 * scale_factor) as u32,
                            (r.bounds.2 as f32 * scale_factor) as u32,
                            (r.bounds.3 as f32 * scale_factor) as u32,
                        ),
                        confidence: r.confidence,
                    })
                    .collect()
            }
            OcrBackend::PaddleOcr => {
                let results = self.process_paddle_ocr(&processed_data, proc_width, proc_height)?;
                let scale_factor = if proc_width != width { width as f32 / proc_width as f32 } else { 1.0 };
                results
                    .into_iter()
                    .map(|r| TextRegion {
                        text: r.text,
                        bounds: (
                            x + (r.bounds.0 as f32 * scale_factor) as u32,
                            y + (r.bounds.1 as f32 * scale_factor) as u32,
                            (r.bounds.2 as f32 * scale_factor) as u32,
                            (r.bounds.3 as f32 * scale_factor) as u32,
                        ),
                        confidence: r.confidence,
                    })
                    .collect()
            }
        };

        let processing_time = start.elapsed();

        Ok(VisionResult {
            text_regions,
            visual_elements: vec![],
            processing_time_ms: processing_time.as_millis() as u64,
        })
    }

    /// Get available Windows OCR languages
    pub fn available_ocr_languages() -> Result<Vec<String>> {
        WindowsOcr::available_languages()
    }
}

/// Result of vision processing on a frame
#[derive(Debug)]
pub struct VisionResult {
    /// Detected text regions
    pub text_regions: Vec<TextRegion>,
    /// Detected visual elements
    pub visual_elements: Vec<VisualElement>,
    /// Processing time in milliseconds
    pub processing_time_ms: u64,
}

/// Convert polygon points to bounding box
fn polygon_to_bounds(polygon: &[(f32, f32)]) -> (u32, u32, u32, u32) {
    if polygon.is_empty() {
        return (0, 0, 0, 0);
    }

    let min_x = polygon.iter().map(|p| p.0).fold(f32::INFINITY, f32::min);
    let min_y = polygon.iter().map(|p| p.1).fold(f32::INFINITY, f32::min);
    let max_x = polygon.iter().map(|p| p.0).fold(f32::NEG_INFINITY, f32::max);
    let max_y = polygon.iter().map(|p| p.1).fold(f32::NEG_INFINITY, f32::max);

    (
        min_x as u32,
        min_y as u32,
        (max_x - min_x) as u32,
        (max_y - min_y) as u32,
    )
}

/// Extract a region from BGRA image data
fn extract_region(
    data: &[u8],
    img_width: u32,
    img_height: u32,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
) -> Vec<u8> {
    let mut region = Vec::with_capacity((width * height * 4) as usize);

    let x = x.min(img_width);
    let y = y.min(img_height);
    let width = width.min(img_width - x);
    let height = height.min(img_height - y);

    for row in y..(y + height) {
        let start = ((row * img_width + x) * 4) as usize;
        let end = start + (width * 4) as usize;
        if end <= data.len() {
            region.extend_from_slice(&data[start..end]);
        }
    }

    region
}
