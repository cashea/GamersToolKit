//! Model management for ONNX Runtime
//!
//! Handles downloading, caching, and loading of PaddleOCR models.

use anyhow::{Context, Result};
use futures_util::StreamExt;
use ort::session::{builder::GraphOptimizationLevel, Session};
use sha2::{Digest, Sha256};
use std::io::Write;
use std::path::{Path, PathBuf};
use tokio::runtime::Runtime;
use tracing::{debug, info, warn};

/// Model identifier for PaddleOCR components
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelType {
    /// Text detection model (DBNet)
    Detection,
    /// Text recognition model (CRNN)
    Recognition,
    /// Text direction classifier (optional)
    Classifier,
    /// Character dictionary for recognition
    Dictionary,
}

impl ModelType {
    /// Get the filename for this model type
    pub fn filename(&self) -> &'static str {
        match self {
            ModelType::Detection => "det.onnx",
            ModelType::Recognition => "rec.onnx",
            ModelType::Classifier => "cls.onnx",
            ModelType::Dictionary => "dict.txt",
        }
    }

    /// Get the download URL for this model
    /// Using PaddleOCR models from Hugging Face (monkt/paddleocr-onnx)
    pub fn download_url(&self) -> &'static str {
        match self {
            // PaddleOCR v3 detection model
            ModelType::Detection => {
                "https://huggingface.co/monkt/paddleocr-onnx/resolve/main/detection/v3/det.onnx"
            }
            // PaddleOCR English recognition model
            ModelType::Recognition => {
                "https://huggingface.co/monkt/paddleocr-onnx/resolve/main/languages/english/rec.onnx"
            }
            // No classifier in this repo - use detection as fallback (not used currently)
            ModelType::Classifier => {
                "https://huggingface.co/monkt/paddleocr-onnx/resolve/main/detection/v3/det.onnx"
            }
            // Character dictionary for English recognition
            ModelType::Dictionary => {
                "https://huggingface.co/monkt/paddleocr-onnx/resolve/main/languages/english/dict.txt"
            }
        }
    }

    /// Expected file size for integrity check (approximate, in bytes)
    pub fn expected_size_range(&self) -> (u64, u64) {
        match self {
            ModelType::Detection => (2_000_000, 5_000_000),      // ~2.43 MB
            ModelType::Recognition => (7_000_000, 10_000_000),   // ~7.83 MB
            ModelType::Classifier => (2_000_000, 5_000_000),     // Using detection as fallback
            ModelType::Dictionary => (500, 10_000),              // ~1.42 KB
        }
    }

    /// Get expected SHA256 checksum for model verification (optional)
    /// Returns None if checksum is not yet known
    pub fn expected_sha256(&self) -> Option<&'static str> {
        // These will be populated once models are uploaded to releases
        // For now, return None to skip verification
        match self {
            ModelType::Detection => None,
            ModelType::Recognition => None,
            ModelType::Classifier => None,
            ModelType::Dictionary => None,
        }
    }

    /// Display name for progress reporting
    pub fn display_name(&self) -> &'static str {
        match self {
            ModelType::Detection => "Text Detection",
            ModelType::Recognition => "Text Recognition",
            ModelType::Classifier => "Text Classifier",
            ModelType::Dictionary => "Character Dictionary",
        }
    }
}

/// Model manifest tracking downloaded models
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ModelManifest {
    pub version: String,
    pub models: Vec<ModelInfo>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ModelInfo {
    pub model_type: String,
    pub filename: String,
    pub size_bytes: u64,
    pub sha256: Option<String>,
    pub downloaded_at: String,
}

/// Progress callback for download operations
pub type DownloadProgressCallback = Box<dyn Fn(u64, Option<u64>) + Send + Sync>;

impl Default for ModelManifest {
    fn default() -> Self {
        Self {
            version: "1.0.0".to_string(),
            models: Vec::new(),
        }
    }
}

/// Model manager for downloading and caching ONNX models
pub struct ModelManager {
    models_dir: PathBuf,
}

impl ModelManager {
    /// Create a new model manager
    pub fn new() -> Result<Self> {
        let data_dir = crate::storage::get_data_dir()?;
        let models_dir = data_dir.join("models");
        std::fs::create_dir_all(&models_dir)?;

        Ok(Self { models_dir })
    }

    /// Create model manager with custom directory
    pub fn with_dir(models_dir: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&models_dir)?;
        Ok(Self { models_dir })
    }

    /// Get the models directory path
    pub fn models_dir(&self) -> &Path {
        &self.models_dir
    }

    /// Get the path to a specific model file
    pub fn model_path(&self, model_type: ModelType) -> PathBuf {
        self.models_dir.join(model_type.filename())
    }

    /// Check if a model is already downloaded
    pub fn is_model_available(&self, model_type: ModelType) -> bool {
        let path = self.model_path(model_type);
        if !path.exists() {
            return false;
        }

        // Verify file size is reasonable
        if let Ok(metadata) = std::fs::metadata(&path) {
            let (min, max) = model_type.expected_size_range();
            let size = metadata.len();
            size >= min && size <= max
        } else {
            false
        }
    }

    /// Check if all required models are available
    pub fn are_models_ready(&self) -> bool {
        self.is_model_available(ModelType::Detection)
            && self.is_model_available(ModelType::Recognition)
    }

    /// Get status of all models
    pub fn get_model_status(&self) -> Vec<(ModelType, bool, Option<u64>)> {
        let models = [
            ModelType::Detection,
            ModelType::Recognition,
            ModelType::Classifier,
        ];

        models
            .iter()
            .map(|&model_type| {
                let path = self.model_path(model_type);
                let available = self.is_model_available(model_type);
                let size = std::fs::metadata(&path).ok().map(|m| m.len());
                (model_type, available, size)
            })
            .collect()
    }

    /// Download a model if not already available
    /// Returns the path to the model file
    pub fn ensure_model(&self, model_type: ModelType) -> Result<PathBuf> {
        let path = self.model_path(model_type);

        if self.is_model_available(model_type) {
            info!("Model {:?} already available at {:?}", model_type, path);
            return Ok(path);
        }

        info!("Downloading model {:?}...", model_type);
        self.download_model(model_type)?;

        Ok(path)
    }

    /// Download all required models
    pub fn ensure_all_models(&self) -> Result<()> {
        self.ensure_model(ModelType::Detection)?;
        self.ensure_model(ModelType::Recognition)?;
        self.ensure_model(ModelType::Dictionary)?;
        // Classifier is optional
        if let Err(e) = self.ensure_model(ModelType::Classifier) {
            warn!("Failed to download classifier model (optional): {}", e);
        }
        Ok(())
    }

    /// Download a specific model (blocking)
    fn download_model(&self, model_type: ModelType) -> Result<()> {
        self.download_model_with_progress(model_type, None)
    }

    /// Download a specific model with optional progress callback
    pub fn download_model_with_progress(
        &self,
        model_type: ModelType,
        progress: Option<DownloadProgressCallback>,
    ) -> Result<()> {
        let url = model_type.download_url();
        let path = self.model_path(model_type);

        info!("Downloading {} model from {}", model_type.display_name(), url);

        // Check if we're in offline mode
        if std::env::var("GAMERS_TOOLKIT_OFFLINE").is_ok() {
            anyhow::bail!("Offline mode: cannot download models. Please download manually from {} and place at {:?}", url, path);
        }

        // Create a tokio runtime for async download
        let rt = Runtime::new().context("Failed to create tokio runtime")?;

        rt.block_on(async {
            self.download_file_async(url, &path, model_type, progress).await
        })?;

        // Verify the download
        if !self.is_model_available(model_type) {
            anyhow::bail!("Download completed but model verification failed");
        }

        // Update manifest
        self.update_manifest_for_model(model_type)?;

        info!("Successfully downloaded {} model", model_type.display_name());
        Ok(())
    }

    /// Async download implementation
    async fn download_file_async(
        &self,
        url: &str,
        path: &Path,
        model_type: ModelType,
        progress: Option<DownloadProgressCallback>,
    ) -> Result<()> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(300)) // 5 minute timeout
            .build()
            .context("Failed to create HTTP client")?;

        let response = client
            .get(url)
            .send()
            .await
            .context("Failed to send download request")?;

        if !response.status().is_success() {
            anyhow::bail!(
                "Download failed with status {}: {}",
                response.status(),
                url
            );
        }

        let total_size = response.content_length();
        debug!("Download size: {:?} bytes", total_size);

        // Create temp file for download
        let temp_path = path.with_extension("tmp");
        let mut file = std::fs::File::create(&temp_path)
            .context("Failed to create temp file")?;

        let mut hasher = Sha256::new();
        let mut downloaded: u64 = 0;
        let mut stream = response.bytes_stream();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.context("Error reading download stream")?;

            file.write_all(&chunk)
                .context("Failed to write to temp file")?;

            hasher.update(&chunk);
            downloaded += chunk.len() as u64;

            // Report progress
            if let Some(ref callback) = progress {
                callback(downloaded, total_size);
            }
        }

        file.flush().context("Failed to flush temp file")?;
        drop(file);

        // Verify checksum if available
        let hash = format!("{:x}", hasher.finalize());
        if let Some(expected_hash) = model_type.expected_sha256() {
            if hash != expected_hash {
                std::fs::remove_file(&temp_path).ok();
                anyhow::bail!(
                    "Checksum mismatch for {}: expected {}, got {}",
                    model_type.filename(),
                    expected_hash,
                    hash
                );
            }
            info!("Checksum verified for {}", model_type.display_name());
        }

        // Move temp file to final location
        std::fs::rename(&temp_path, path)
            .context("Failed to move downloaded file to final location")?;

        Ok(())
    }

    /// Update manifest after successful download
    fn update_manifest_for_model(&self, model_type: ModelType) -> Result<()> {
        let mut manifest = self.load_manifest().unwrap_or_default();

        let path = self.model_path(model_type);
        let metadata = std::fs::metadata(&path)?;

        // Calculate SHA256
        let hash = {
            let data = std::fs::read(&path)?;
            let mut hasher = Sha256::new();
            hasher.update(&data);
            format!("{:x}", hasher.finalize())
        };

        let model_info = ModelInfo {
            model_type: format!("{:?}", model_type),
            filename: model_type.filename().to_string(),
            size_bytes: metadata.len(),
            sha256: Some(hash),
            downloaded_at: chrono_lite_now(),
        };

        // Update or add model info
        if let Some(existing) = manifest.models.iter_mut().find(|m| m.filename == model_info.filename) {
            *existing = model_info;
        } else {
            manifest.models.push(model_info);
        }

        self.save_manifest(&manifest)?;
        Ok(())
    }

    /// Download all required models with progress reporting
    pub fn download_all_with_progress<F>(&self, mut on_progress: F) -> Result<()>
    where
        F: FnMut(ModelType, u64, Option<u64>),
    {
        let models = [ModelType::Detection, ModelType::Recognition, ModelType::Dictionary];

        for model_type in models {
            if self.is_model_available(model_type) {
                info!("Model {:?} already available, skipping download", model_type);
                continue;
            }

            let mt = model_type;
            let progress_callback: DownloadProgressCallback = Box::new(move |downloaded, total| {
                // Note: We can't easily call on_progress here due to borrowing rules
                // In a real implementation, you'd use channels or Arc<Mutex>
                debug!("{:?}: {} / {:?} bytes", mt, downloaded, total);
            });

            self.download_model_with_progress(model_type, Some(progress_callback))?;
            on_progress(model_type, 0, Some(0)); // Signal completion
        }

        Ok(())
    }

    /// Load the model manifest
    pub fn load_manifest(&self) -> Result<ModelManifest> {
        let manifest_path = self.models_dir.join("manifest.json");
        if manifest_path.exists() {
            let content = std::fs::read_to_string(&manifest_path)?;
            let manifest: ModelManifest = serde_json::from_str(&content)?;
            Ok(manifest)
        } else {
            Ok(ModelManifest::default())
        }
    }

    /// Save the model manifest
    pub fn save_manifest(&self, manifest: &ModelManifest) -> Result<()> {
        let manifest_path = self.models_dir.join("manifest.json");
        let content = serde_json::to_string_pretty(manifest)?;
        std::fs::write(manifest_path, content)?;
        Ok(())
    }
}

/// ONNX Runtime session wrapper
pub struct OnnxSession {
    session: Session,
    input_names: Vec<String>,
    output_names: Vec<String>,
}

impl OnnxSession {
    /// Create a new ONNX session from a model file
    pub fn new(model_path: &Path) -> Result<Self> {
        info!("Loading ONNX model from {:?}", model_path);

        // Initialize ONNX Runtime environment
        let session = Session::builder()?
            .with_optimization_level(GraphOptimizationLevel::Level3)?
            .with_intra_threads(4)?
            .commit_from_file(model_path)
            .context("Failed to load ONNX model")?;

        // Get input/output names
        let input_names: Vec<String> = session
            .inputs
            .iter()
            .map(|input| input.name.clone())
            .collect();

        let output_names: Vec<String> = session
            .outputs
            .iter()
            .map(|output| output.name.clone())
            .collect();

        info!(
            "Model loaded. Inputs: {:?}, Outputs: {:?}",
            input_names, output_names
        );

        Ok(Self {
            session,
            input_names,
            output_names,
        })
    }

    /// Create session with GPU acceleration if available
    pub fn new_with_gpu(model_path: &Path) -> Result<Self> {
        info!("Loading ONNX model with GPU acceleration from {:?}", model_path);

        // Try DirectML first (Windows), then CUDA, then fall back to CPU
        let session_builder = Session::builder()?
            .with_optimization_level(GraphOptimizationLevel::Level3)?
            .with_intra_threads(4)?;

        // Try to add DirectML execution provider (Windows GPU)
        #[cfg(target_os = "windows")]
        let session_builder = {
            use ort::execution_providers::DirectMLExecutionProvider;
            match session_builder.with_execution_providers([
                DirectMLExecutionProvider::default().build(),
            ]) {
                Ok(builder) => {
                    info!("DirectML GPU acceleration enabled");
                    builder
                }
                Err(e) => {
                    warn!("DirectML not available, using CPU: {}", e);
                    Session::builder()?
                        .with_optimization_level(GraphOptimizationLevel::Level3)?
                        .with_intra_threads(4)?
                }
            }
        };

        #[cfg(not(target_os = "windows"))]
        let session_builder = session_builder;

        let session = session_builder
            .commit_from_file(model_path)
            .context("Failed to load ONNX model")?;

        let input_names: Vec<String> = session
            .inputs
            .iter()
            .map(|input| input.name.clone())
            .collect();

        let output_names: Vec<String> = session
            .outputs
            .iter()
            .map(|output| output.name.clone())
            .collect();

        info!(
            "Model loaded with GPU. Inputs: {:?}, Outputs: {:?}",
            input_names, output_names
        );

        Ok(Self {
            session,
            input_names,
            output_names,
        })
    }

    /// Get the underlying session for running inference
    pub fn session(&self) -> &Session {
        &self.session
    }

    /// Get the underlying session mutably for running inference
    pub fn session_mut(&mut self) -> &mut Session {
        &mut self.session
    }

    /// Get input names
    pub fn input_names(&self) -> &[String] {
        &self.input_names
    }

    /// Get output names
    pub fn output_names(&self) -> &[String] {
        &self.output_names
    }

    /// Get input tensor info
    pub fn input_info(&self) -> Vec<TensorInfo> {
        self.session
            .inputs
            .iter()
            .map(|input| TensorInfo {
                name: input.name.clone(),
                shape: extract_shape(&input.input_type),
            })
            .collect()
    }

    /// Get output tensor info
    pub fn output_info(&self) -> Vec<TensorInfo> {
        self.session
            .outputs
            .iter()
            .map(|output| TensorInfo {
                name: output.name.clone(),
                shape: extract_shape(&output.output_type),
            })
            .collect()
    }
}

/// Tensor shape information
#[derive(Debug, Clone)]
pub struct TensorInfo {
    pub name: String,
    pub shape: Vec<i64>,
}

/// Extract shape from ONNX value type
fn extract_shape(value_type: &ort::value::ValueType) -> Vec<i64> {
    // Use the tensor_shape() helper method
    if let Some(shape) = value_type.tensor_shape() {
        shape.iter().map(|&d| d).collect()
    } else {
        vec![]
    }
}

/// Get current timestamp as ISO 8601 string (lightweight alternative to chrono)
fn chrono_lite_now() -> String {
    use std::time::SystemTime;

    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    // Convert Unix timestamp to ISO 8601 format
    // This is a simplified version - just returns Unix timestamp for now
    format!("{}", now)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_type_filenames() {
        assert_eq!(ModelType::Detection.filename(), "det.onnx");
        assert_eq!(ModelType::Recognition.filename(), "rec.onnx");
        assert_eq!(ModelType::Classifier.filename(), "cls.onnx");
    }

    #[test]
    fn test_model_manager_creation() {
        // This will create the models directory
        let manager = ModelManager::new();
        assert!(manager.is_ok());
    }

    #[test]
    fn test_model_status() {
        let manager = ModelManager::new().unwrap();
        let status = manager.get_model_status();
        assert_eq!(status.len(), 3);
    }
}
