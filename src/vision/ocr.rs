//! OCR (Optical Character Recognition) module
//!
//! Uses PaddleOCR models via ONNX Runtime for text detection and recognition.

use anyhow::{Context, Result};
use ndarray::{Array2, Array3};
use std::path::Path;
use tracing::{debug, info, warn};

use super::models::OnnxSession;
use super::preprocess::{
    rgba_to_rgb_f32, crop_polygon, preprocess_for_detection,
    preprocess_for_recognition, PreprocessConfig,
};

/// OCR engine using PaddleOCR via ONNX Runtime
pub struct OcrEngine {
    detection_session: OnnxSession,
    recognition_session: OnnxSession,
    preprocess_config: PreprocessConfig,
    vocabulary: Vec<char>,
    detection_threshold: f32,
    recognition_threshold: f32,
}

impl OcrEngine {
    /// Initialize OCR engine with model paths
    pub fn new(detection_model: &str, recognition_model: &str, use_gpu: bool) -> Result<Self> {
        // Use default dictionary path (same directory as models)
        let dict_path = Path::new(recognition_model)
            .parent()
            .unwrap_or(Path::new("."))
            .join("dict.txt");
        Self::new_with_dict(detection_model, recognition_model, dict_path.to_str().unwrap_or("dict.txt"), use_gpu)
    }

    /// Initialize OCR engine with model paths and custom dictionary
    pub fn new_with_dict(detection_model: &str, recognition_model: &str, dict_path: &str, use_gpu: bool) -> Result<Self> {
        info!("Initializing OCR engine...");
        info!("  Detection model: {}", detection_model);
        info!("  Recognition model: {}", recognition_model);
        info!("  Dictionary: {}", dict_path);
        info!("  GPU acceleration: {}", use_gpu);

        let detection_session = if use_gpu {
            OnnxSession::new_with_gpu(std::path::Path::new(detection_model))
                .or_else(|e| {
                    warn!("Failed to load detection model with GPU, falling back to CPU: {}", e);
                    OnnxSession::new(std::path::Path::new(detection_model))
                })?
        } else {
            OnnxSession::new(std::path::Path::new(detection_model))?
        };

        let recognition_session = if use_gpu {
            OnnxSession::new_with_gpu(std::path::Path::new(recognition_model))
                .or_else(|e| {
                    warn!("Failed to load recognition model with GPU, falling back to CPU: {}", e);
                    OnnxSession::new(std::path::Path::new(recognition_model))
                })?
        } else {
            OnnxSession::new(std::path::Path::new(recognition_model))?
        };

        // Load vocabulary from dictionary file
        let vocabulary = Self::load_vocabulary(dict_path)?;
        info!("  Vocabulary size: {} characters", vocabulary.len());

        info!("OCR engine initialized successfully");
        info!("  Detection inputs: {:?}", detection_session.input_info());
        info!("  Recognition inputs: {:?}", recognition_session.input_info());
        info!("  Detection outputs: {:?}", detection_session.output_info());

        Ok(Self {
            detection_session,
            recognition_session,
            preprocess_config: PreprocessConfig::default(),
            vocabulary,
            detection_threshold: 0.3,
            recognition_threshold: 0.001, // Very low threshold - PaddleOCR ONNX can have low confidence
        })
    }

    /// Load vocabulary from dictionary file
    fn load_vocabulary(dict_path: &str) -> Result<Vec<char>> {
        let content = std::fs::read_to_string(dict_path)
            .with_context(|| format!("Failed to read dictionary file: {}", dict_path))?;

        // Each line contains one character
        // PaddleOCR dict format: characters only, blank token is implicit at the END (index = vocab_size)
        let vocabulary: Vec<char> = content
            .lines()
            .filter_map(|line| line.chars().next())
            .collect();

        info!("Loaded {} characters from dictionary", vocabulary.len());
        info!("First 20 chars: {:?}", &vocabulary[..vocabulary.len().min(20)]);
        info!("Last 5 chars: {:?}", &vocabulary[vocabulary.len().saturating_sub(5)..]);
        // Check for space character
        for (i, &c) in vocabulary.iter().enumerate() {
            if c == ' ' {
                info!("Found SPACE character at vocabulary index {}", i);
            }
        }

        Ok(vocabulary)
    }

    /// Set detection confidence threshold
    pub fn set_detection_threshold(&mut self, threshold: f32) {
        self.detection_threshold = threshold.clamp(0.0, 1.0);
    }

    /// Set recognition confidence threshold
    pub fn set_recognition_threshold(&mut self, threshold: f32) {
        self.recognition_threshold = threshold.clamp(0.0, 1.0);
    }

    /// Run OCR on an image buffer (BGRA format)
    pub fn recognize(&mut self, image_data: &[u8], width: u32, height: u32) -> Result<Vec<OcrResult>> {
        if image_data.is_empty() || width == 0 || height == 0 {
            return Ok(vec![]);
        }

        info!("Running OCR on {}x{} image", width, height);

        // Step 1: Detect text regions
        let detections = self.detect(image_data, width, height)?;
        info!("Detected {} text regions", detections.len());

        if detections.is_empty() {
            return Ok(vec![]);
        }

        // Step 2: Recognize text in each region
        let rgb_image = rgba_to_rgb_f32(image_data, width, height);
        let mut results = Vec::with_capacity(detections.len());

        for detection in detections {
            if let Some(text_result) = self.recognize_region(&rgb_image, &detection)? {
                if text_result.confidence >= self.recognition_threshold {
                    results.push(text_result);
                }
            }
        }

        debug!("Recognized {} text regions", results.len());
        Ok(results)
    }

    /// Detect text regions in an image
    fn detect(&mut self, image_data: &[u8], width: u32, height: u32) -> Result<Vec<DetectedRegion>> {
        // Preprocess image for detection
        let (input_tensor, scale) = preprocess_for_detection(
            image_data,
            width,
            height,
            &self.preprocess_config,
        );

        // Create ONNX tensor from ndarray
        let input_value = ort::value::Tensor::from_array(input_tensor)?;

        // Run detection inference
        let outputs = self.detection_session.session_mut().run(
            ort::inputs![input_value]
        ).context("Detection inference failed")?;

        // Get output tensor (probability map)
        let output = outputs.iter().next()
            .ok_or_else(|| anyhow::anyhow!("No detection output"))?;

        // Extract tensor data as (shape, data) tuple and clone to release borrow
        let (shape, data) = output.1.try_extract_tensor::<f32>()
            .context("Failed to extract detection output")?;

        // Clone data to release borrow on outputs/self
        let shape_vec: Vec<i64> = shape.iter().map(|&d| d).collect();
        let data_vec: Vec<f32> = data.to_vec();

        // Drop outputs to release borrow
        drop(outputs);

        // Post-process: extract bounding boxes from probability map
        let detections = self.postprocess_detection_raw(&shape_vec, &data_vec, scale, width, height)?;

        Ok(detections)
    }

    /// Post-process detection output to get bounding boxes from raw tensor data
    fn postprocess_detection_raw(
        &self,
        shape: &[i64],
        data: &[f32],
        scale: f32,
        orig_width: u32,
        orig_height: u32,
    ) -> Result<Vec<DetectedRegion>> {
        info!("Detection output shape: {:?}, data len: {}", shape, data.len());

        if shape.len() < 4 {
            warn!("Shape too short ({}), returning empty", shape.len());
            return Ok(vec![]);
        }

        let h = shape[2] as usize;
        let w = shape[3] as usize;

        // Analyze the probability map values
        let mut min_val = f32::MAX;
        let mut max_val = f32::MIN;
        let mut sum_val = 0.0f64;
        let mut above_threshold = 0usize;

        for &val in data.iter() {
            min_val = min_val.min(val);
            max_val = max_val.max(val);
            sum_val += val as f64;
            if val > self.detection_threshold {
                above_threshold += 1;
            }
        }

        let avg_val = if !data.is_empty() { sum_val / data.len() as f64 } else { 0.0 };
        info!(
            "Detection map stats - min: {:.4}, max: {:.4}, avg: {:.4}, above threshold({}): {}",
            min_val, max_val, avg_val, self.detection_threshold, above_threshold
        );

        // Threshold the probability map
        let mut binary_map = Array2::<u8>::zeros((h, w));
        for y in 0..h {
            for x in 0..w {
                let idx = y * w + x; // Assume NCHW format with N=1, C=1
                if idx < data.len() && data[idx] > self.detection_threshold {
                    binary_map[[y, x]] = 1;
                }
            }
        }

        // Find connected components (simplified - in production use imageproc)
        let boxes = find_text_boxes(&binary_map, self.detection_threshold);
        debug!("Found {} bounding boxes from binary map", boxes.len());

        // Scale boxes back to original image coordinates
        let mut detections = Vec::new();
        for bbox in boxes {
            let polygon = [
                (bbox.0 as f32 / scale, bbox.1 as f32 / scale),
                (bbox.2 as f32 / scale, bbox.1 as f32 / scale),
                (bbox.2 as f32 / scale, bbox.3 as f32 / scale),
                (bbox.0 as f32 / scale, bbox.3 as f32 / scale),
            ];

            // Clamp to image bounds
            let polygon = polygon.map(|(x, y)| {
                (
                    x.max(0.0).min(orig_width as f32 - 1.0),
                    y.max(0.0).min(orig_height as f32 - 1.0),
                )
            });

            detections.push(DetectedRegion {
                polygon,
                confidence: 1.0, // Simplified
            });
        }

        Ok(detections)
    }

    /// Recognize text in a detected region
    fn recognize_region(
        &mut self,
        image: &Array3<f32>,
        region: &DetectedRegion,
    ) -> Result<Option<OcrResult>> {
        // Crop the region
        let cropped = crop_polygon(image, &region.polygon);

        let (h, w, _) = cropped.dim();
        debug!("Cropped region size: {}x{}, polygon: {:?}", w, h, region.polygon);
        if h < 2 || w < 2 {
            debug!("Region too small, skipping");
            return Ok(None);
        }

        // Preprocess for recognition
        let input_tensor = preprocess_for_recognition(&cropped, &self.preprocess_config);
        debug!("Recognition input tensor shape: {:?}", input_tensor.dim());

        // Create ONNX tensor from ndarray
        let input_value = ort::value::Tensor::from_array(input_tensor)?;

        // Run recognition inference
        let outputs = self.recognition_session.session_mut().run(
            ort::inputs![input_value]
        ).context("Recognition inference failed")?;

        // Get output tensor
        let output = outputs.iter().next()
            .ok_or_else(|| anyhow::anyhow!("No recognition output"))?;
        let (shape, data) = output.1.try_extract_tensor::<f32>()
            .context("Failed to extract recognition output")?;

        // Clone data to release borrow on outputs/self
        let shape_vec: Vec<i64> = shape.iter().map(|&d| d).collect();
        let data_vec: Vec<f32> = data.to_vec();

        // Drop outputs to release borrow
        drop(outputs);

        // Decode output using CTC
        debug!("Recognition output shape: {:?}, data len: {}", shape_vec, data_vec.len());
        let (text, confidence) = self.ctc_decode_raw(&shape_vec, &data_vec)?;
        debug!("CTC decoded: text='{}', confidence={:.3}", text, confidence);

        if text.is_empty() {
            debug!("Empty text after CTC decode");
            return Ok(None);
        }

        Ok(Some(OcrResult {
            text,
            polygon: region.polygon.to_vec(),
            confidence,
        }))
    }

    /// CTC decoding for recognition output from raw tensor data
    fn ctc_decode_raw(&self, shape: &[i64], data: &[f32]) -> Result<(String, f32)> {
        if shape.len() < 3 {
            return Ok((String::new(), 0.0));
        }

        let seq_len = shape[1] as usize;
        let vocab_size = shape[2] as usize;

        // Debug: analyze output statistics for first time step
        if seq_len > 0 {
            let mut min_val = f32::MAX;
            let mut max_val = f32::MIN;
            for v in 0..vocab_size.min(10) {
                let val = data[v];
                min_val = min_val.min(val);
                max_val = max_val.max(val);
            }
            debug!("Recognition logits t=0: min={:.4}, max={:.4} (first 10 classes)", min_val, max_val);
        }

        // PaddleOCR ONNX model: 438 classes
        // - Index 0 is the blank/CTC token
        // - Indices 1-436 map to dict.txt characters (0-9, A-Z, a-z, symbols)
        // - Index 437 may be end token or padding
        let blank_idx = 0;

        let mut text = String::new();
        let mut total_confidence = 0.0;
        let mut prev_char_idx: Option<usize> = None; // Track last non-blank character
        let mut count = 0;

        for t in 0..seq_len {
            // Find argmax
            let mut max_idx = 0;
            let mut max_val = f32::NEG_INFINITY;

            for v in 0..vocab_size {
                let idx = t * vocab_size + v;
                if idx < data.len() {
                    let val = data[idx];
                    if val > max_val {
                        max_val = val;
                        max_idx = v;
                    }
                }
            }

            // Skip blank token (index 0) and potential end token (last index)
            if max_idx == blank_idx || max_idx >= vocab_size - 1 {
                // Blank resets the "previous character" so repeated chars after blank are allowed
                prev_char_idx = None;
                continue;
            }

            // The model outputs probabilities directly (already softmaxed)
            // Just use the max value as the confidence
            let confidence = max_val;

            // CTC: skip repeated characters (only consecutive non-blanks)
            // Emit a character when it's different from the previous non-blank
            if Some(max_idx) != prev_char_idx {
                // Model index maps directly to vocabulary index
                // Model index 0 = CTC blank (already skipped above)
                // Model indices 1-437 map to vocabulary indices 0-436
                // But dict.txt has space at index 0, so model index 1 -> vocab[0] = space
                // We need to map model index to vocab index by subtracting 1
                let vocab_idx = max_idx - 1;
                if vocab_idx < self.vocabulary.len() {
                    let ch = self.vocabulary[vocab_idx];
                    debug!("t={}: max_idx={}, vocab_idx={}, char='{}', conf={:.4}, logit={:.4}",
                           t, max_idx, vocab_idx, ch, confidence, max_val);
                    // Skip space characters in output (they appear between words)
                    if ch != ' ' {
                        text.push(ch);
                        total_confidence += confidence;
                        count += 1;
                    }
                }
            }

            prev_char_idx = Some(max_idx);
        }

        let avg_confidence = if count > 0 {
            total_confidence / count as f32
        } else {
            0.0
        };

        // Trim whitespace
        let text = text.trim().to_string();

        Ok((text, avg_confidence))
    }
}

/// Detected text region before recognition
#[derive(Debug, Clone)]
struct DetectedRegion {
    polygon: [(f32, f32); 4],
    confidence: f32,
}

/// Single OCR detection result
#[derive(Debug, Clone)]
pub struct OcrResult {
    /// Recognized text
    pub text: String,
    /// Bounding polygon points
    pub polygon: Vec<(f32, f32)>,
    /// Recognition confidence
    pub confidence: f32,
}

/// Find text bounding boxes from binary map (simplified connected components)
fn find_text_boxes(binary_map: &Array2<u8>, _threshold: f32) -> Vec<(usize, usize, usize, usize)> {
    let (h, w) = binary_map.dim();
    let mut boxes = Vec::new();

    // Simple approach: find min/max bounds of each connected region
    // In production, use proper connected component labeling

    let mut visited = Array2::<bool>::from_elem((h, w), false);

    for start_y in 0..h {
        for start_x in 0..w {
            if binary_map[[start_y, start_x]] == 1 && !visited[[start_y, start_x]] {
                // Found a new region - flood fill to find bounds
                let mut min_x = start_x;
                let mut min_y = start_y;
                let mut max_x = start_x;
                let mut max_y = start_y;

                let mut stack = vec![(start_y, start_x)];

                while let Some((y, x)) = stack.pop() {
                    if y >= h || x >= w || visited[[y, x]] || binary_map[[y, x]] == 0 {
                        continue;
                    }

                    visited[[y, x]] = true;
                    min_x = min_x.min(x);
                    min_y = min_y.min(y);
                    max_x = max_x.max(x);
                    max_y = max_y.max(y);

                    // Add neighbors
                    if y > 0 { stack.push((y - 1, x)); }
                    if y < h - 1 { stack.push((y + 1, x)); }
                    if x > 0 { stack.push((y, x - 1)); }
                    if x < w - 1 { stack.push((y, x + 1)); }
                }

                // Filter small boxes
                let box_w = max_x - min_x;
                let box_h = max_y - min_y;
                if box_w >= 4 && box_h >= 4 {
                    boxes.push((min_x, min_y, max_x, max_y));
                }
            }
        }
    }

    boxes
}

/// Compute softmax probability for the given class index
fn softmax_prob(data: &[f32], t: usize, vocab_size: usize, class_idx: usize) -> f32 {
    let offset = t * vocab_size;

    // Find max for numerical stability
    let mut max_val = f32::NEG_INFINITY;
    for v in 0..vocab_size {
        let idx = offset + v;
        if idx < data.len() && data[idx] > max_val {
            max_val = data[idx];
        }
    }

    // Compute softmax denominator
    let mut sum_exp = 0.0f32;
    for v in 0..vocab_size {
        let idx = offset + v;
        if idx < data.len() {
            sum_exp += (data[idx] - max_val).exp();
        }
    }

    // Compute probability for the target class
    let target_idx = offset + class_idx;
    if target_idx < data.len() && sum_exp > 0.0 {
        (data[target_idx] - max_val).exp() / sum_exp
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    // Note: ENGLISH_CHARS constant was removed in a previous refactor
    // This test is currently disabled until vocabulary management is revisited
    // #[test]
    // fn test_vocabulary() {
    //     let vocab: Vec<char> = ENGLISH_CHARS.chars().collect();
    //     assert!(vocab.len() > 90); // Should have printable ASCII
    //     assert!(vocab.contains(&'A'));
    //     assert!(vocab.contains(&'0'));
    // }
}
