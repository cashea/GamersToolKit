# Vision Layer TODO

Detailed implementation tasks for the OCR and template matching system.

---

## 1. ONNX Runtime Setup

### Dependencies
- [ ] Add `ort = "2.x"` to Cargo.toml
- [ ] Add `ndarray = "0.16"` for tensor operations
- [ ] Add `imageproc = "0.25"` for preprocessing
- [ ] Configure feature flags for CPU/CUDA/DirectML

### Initialization
- [ ] Create `OnnxSession` wrapper struct
- [ ] Implement session builder with provider selection
- [ ] Add GPU detection and fallback logic
- [ ] Implement session pooling for multi-threaded access

---

## 2. PaddleOCR Integration

### Model Files
- [ ] Define model file paths (configurable)
- [ ] Implement model download from release assets
- [ ] Add model integrity verification (checksum)
- [ ] Create model version manifest

### Detection Model (`det`)
- [ ] Load detection ONNX model
- [ ] Implement input preprocessing:
  - [ ] Resize to model input size
  - [ ] Normalize pixel values
  - [ ] Convert to NCHW tensor format
- [ ] Run inference
- [ ] Post-process output:
  - [ ] Threshold probability map
  - [ ] Extract bounding polygons
  - [ ] Apply NMS (Non-Maximum Suppression)

### Recognition Model (`rec`)
- [ ] Load recognition ONNX model
- [ ] Implement input preprocessing:
  - [ ] Crop detected regions
  - [ ] Resize to fixed height
  - [ ] Normalize and pad
- [ ] Run inference
- [ ] Decode output:
  - [ ] CTC decoding
  - [ ] Character vocabulary mapping
  - [ ] Confidence scoring

### Direction Classifier (`cls`) - Optional
- [ ] Load classifier model
- [ ] Detect text orientation
- [ ] Rotate regions as needed

---

## 3. OCR Pipeline

### Frame Processing
- [ ] Convert BGRA frame to RGB
- [ ] Implement ROI extraction from profile
- [ ] Create processing queue with priority

### Text Detection
- [ ] Run detection on frame/ROI
- [ ] Filter by minimum confidence
- [ ] Filter by minimum size
- [ ] Cache detection results

### Text Recognition
- [ ] Crop regions from detection
- [ ] Batch process multiple regions
- [ ] Aggregate results with positions

### Result Caching
- [ ] Implement result caching by frame hash
- [ ] Add TTL-based cache expiration
- [ ] Track OCR latency metrics

---

## 4. Template Matching

### Template Loading
- [ ] Define template asset format (PNG with metadata)
- [ ] Load templates from profile directory
- [ ] Support multiple scales per template
- [ ] Generate template pyramids

### Matching Algorithm
- [ ] Implement normalized cross-correlation
- [ ] Add multi-scale matching
- [ ] Support rotation invariance (optional)
- [ ] Threshold-based detection

### Optimization
- [ ] GPU-accelerated matching (optional)
- [ ] Region-of-interest limiting
- [ ] Template hash for quick rejection

---

## 5. API Design

### Public Interface
```rust
// Target API design
pub struct VisionEngine {
    ocr: OcrEngine,
    templates: TemplateEngine,
}

impl VisionEngine {
    pub fn new(config: VisionConfig) -> Result<Self>;
    pub fn process_frame(&self, frame: &CapturedFrame) -> VisionResult;
    pub fn process_region(&self, frame: &CapturedFrame, roi: Rect) -> VisionResult;
}

pub struct VisionResult {
    pub text_detections: Vec<OcrResult>,
    pub template_matches: Vec<TemplateMatch>,
    pub processing_time_ms: u64,
}
```

### Configuration
- [ ] Define `VisionConfig` struct
- [ ] Model path configuration
- [ ] Confidence thresholds
- [ ] Performance tuning options

---

## 6. Testing

### Unit Tests
- [ ] Model loading tests
- [ ] Preprocessing tests
- [ ] Postprocessing tests

### Integration Tests
- [ ] End-to-end OCR test with sample images
- [ ] Template matching test with sample icons
- [ ] Performance benchmark tests

### Test Assets
- [ ] Create test image dataset
- [ ] Include various fonts/sizes
- [ ] Include game screenshot samples

---

## Performance Targets

| Metric | Target |
|--------|--------|
| Detection latency | < 30ms |
| Recognition latency | < 20ms per region |
| Total OCR pipeline | < 50ms per frame |
| Memory usage | < 200MB for models |
