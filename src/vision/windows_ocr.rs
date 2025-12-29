//! Windows OCR API backend
//!
//! Uses the built-in Windows OCR (Media.Ocr) for fast, accurate text recognition.
//! This is particularly good for screen text and game UI.

use anyhow::{Context, Result};
use tracing::{debug, info, warn};
use windows::{
    core::HSTRING,
    Foundation::IAsyncOperation,
    Globalization::Language,
    Graphics::Imaging::{BitmapPixelFormat, SoftwareBitmap},
    Media::Ocr::{OcrEngine as WinOcrEngine, OcrResult as WinOcrResult},
};

/// OCR result from Windows OCR
#[derive(Debug, Clone)]
pub struct WindowsOcrResult {
    /// Recognized text
    pub text: String,
    /// Bounding box (x, y, width, height)
    pub bounds: (u32, u32, u32, u32),
    /// Word-level confidence (Windows OCR doesn't provide per-word confidence, so this is 1.0)
    pub confidence: f32,
}

/// Windows OCR engine wrapper
pub struct WindowsOcr {
    engine: WinOcrEngine,
    language: String,
}

impl WindowsOcr {
    /// Create a new Windows OCR engine with the specified language
    pub fn new(language_tag: &str) -> Result<Self> {
        info!("Initializing Windows OCR engine with language: {}", language_tag);

        let language = Language::CreateLanguage(&HSTRING::from(language_tag))
            .context("Failed to create language")?;

        // Check if this language is supported
        if !WinOcrEngine::IsLanguageSupported(&language)
            .context("Failed to check language support")?
        {
            warn!("Language '{}' not supported, falling back to system default", language_tag);
            let engine = WinOcrEngine::TryCreateFromUserProfileLanguages()
                .context("Failed to create OCR engine from user profile")?;

            let recognizer_lang = engine.RecognizerLanguage()
                .context("Failed to get recognizer language")?;
            let lang_tag = recognizer_lang.LanguageTag()
                .context("Failed to get language tag")?
                .to_string();

            info!("Windows OCR initialized with language: {}", lang_tag);
            return Ok(Self {
                engine,
                language: lang_tag,
            });
        }

        let engine = WinOcrEngine::TryCreateFromLanguage(&language)
            .context("Failed to create OCR engine for language")?;

        info!("Windows OCR initialized successfully");

        Ok(Self {
            engine,
            language: language_tag.to_string(),
        })
    }

    /// Create a Windows OCR engine with the default system language
    pub fn new_default() -> Result<Self> {
        Self::new("en-US")
    }

    /// Get the current language
    pub fn language(&self) -> &str {
        &self.language
    }

    /// Get available OCR languages on this system
    pub fn available_languages() -> Result<Vec<String>> {
        let languages = WinOcrEngine::AvailableRecognizerLanguages()
            .context("Failed to get available languages")?;

        let mut result = Vec::new();
        for i in 0..languages.Size().context("Failed to get languages size")? {
            if let Ok(lang) = languages.GetAt(i) {
                if let Ok(tag) = lang.LanguageTag() {
                    result.push(tag.to_string());
                }
            }
        }

        Ok(result)
    }

    /// Recognize text in an RGBA image buffer
    pub fn recognize(&self, image_data: &[u8], width: u32, height: u32) -> Result<Vec<WindowsOcrResult>> {
        if image_data.is_empty() || width == 0 || height == 0 {
            return Ok(vec![]);
        }

        debug!("Windows OCR: Processing {}x{} image", width, height);

        // Convert RGBA to BGRA (Windows expects BGRA)
        let bgra_data = rgba_to_bgra(image_data);

        // Create SoftwareBitmap from the image data
        let bitmap = create_software_bitmap(&bgra_data, width, height)?;

        // Run OCR
        let ocr_result = run_ocr_sync(&self.engine, &bitmap)?;

        // Extract results
        let results = extract_results(&ocr_result)?;

        debug!("Windows OCR: Found {} text regions", results.len());

        Ok(results)
    }

    /// Get the full text from an image (convenience method)
    pub fn recognize_text(&self, image_data: &[u8], width: u32, height: u32) -> Result<String> {
        if image_data.is_empty() || width == 0 || height == 0 {
            return Ok(String::new());
        }

        // Convert RGBA to BGRA
        let bgra_data = rgba_to_bgra(image_data);

        // Create SoftwareBitmap
        let bitmap = create_software_bitmap(&bgra_data, width, height)?;

        // Run OCR
        let ocr_result = run_ocr_sync(&self.engine, &bitmap)?;

        // Get full text
        let text = ocr_result.Text()
            .context("Failed to get OCR text")?
            .to_string();

        Ok(text)
    }
}

/// Convert RGBA to BGRA (Windows expects BGRA)
fn rgba_to_bgra(rgba: &[u8]) -> Vec<u8> {
    let mut bgra = rgba.to_vec();
    for chunk in bgra.chunks_exact_mut(4) {
        chunk.swap(0, 2); // Swap R and B
    }
    bgra
}

/// Create a SoftwareBitmap from BGRA data using CopyFromBuffer
fn create_software_bitmap(bgra_data: &[u8], width: u32, height: u32) -> Result<SoftwareBitmap> {
    use windows::Storage::Streams::{DataWriter, InMemoryRandomAccessStream};

    // Create an in-memory stream and write the pixel data
    let stream = InMemoryRandomAccessStream::new()
        .context("Failed to create in-memory stream")?;

    let writer = DataWriter::CreateDataWriter(&stream)
        .context("Failed to create data writer")?;

    writer.WriteBytes(bgra_data)
        .context("Failed to write pixel data")?;

    writer.StoreAsync()
        .context("Failed to start store operation")?
        .get()
        .context("Failed to store data")?;

    writer.FlushAsync()
        .context("Failed to start flush operation")?
        .get()
        .context("Failed to flush data")?;

    // Reset stream position
    stream.Seek(0)
        .context("Failed to seek stream")?;

    // Create bitmap
    let bitmap = SoftwareBitmap::Create(
        BitmapPixelFormat::Bgra8,
        width as i32,
        height as i32,
    ).context("Failed to create SoftwareBitmap")?;

    // Get input stream and create buffer
    let input_stream = stream.GetInputStreamAt(0)
        .context("Failed to get input stream")?;

    let reader = windows::Storage::Streams::DataReader::CreateDataReader(&input_stream)
        .context("Failed to create data reader")?;

    reader.LoadAsync(bgra_data.len() as u32)
        .context("Failed to start load operation")?
        .get()
        .context("Failed to load data")?;

    let buffer = reader.ReadBuffer(bgra_data.len() as u32)
        .context("Failed to read buffer")?;

    // Copy from buffer to bitmap
    bitmap.CopyFromBuffer(&buffer)
        .context("Failed to copy buffer to bitmap")?;

    Ok(bitmap)
}

/// Run OCR synchronously (blocks until complete)
fn run_ocr_sync(engine: &WinOcrEngine, bitmap: &SoftwareBitmap) -> Result<WinOcrResult> {
    let async_op: IAsyncOperation<WinOcrResult> = engine.RecognizeAsync(bitmap)
        .context("Failed to start OCR recognition")?;

    // Block until the operation completes
    let result = async_op.get()
        .context("OCR recognition failed")?;

    Ok(result)
}

/// Extract text regions from OCR result
fn extract_results(ocr_result: &WinOcrResult) -> Result<Vec<WindowsOcrResult>> {
    let mut results = Vec::new();

    let lines = ocr_result.Lines()
        .context("Failed to get OCR lines")?;

    for i in 0..lines.Size().context("Failed to get lines size")? {
        let line = lines.GetAt(i)
            .context("Failed to get line")?;

        let words = line.Words()
            .context("Failed to get words")?;

        for j in 0..words.Size().context("Failed to get words size")? {
            let word = words.GetAt(j)
                .context("Failed to get word")?;

            let text = word.Text()
                .context("Failed to get word text")?
                .to_string();

            let rect = word.BoundingRect()
                .context("Failed to get bounding rect")?;

            results.push(WindowsOcrResult {
                text,
                bounds: (
                    rect.X as u32,
                    rect.Y as u32,
                    rect.Width as u32,
                    rect.Height as u32,
                ),
                confidence: 1.0, // Windows OCR doesn't provide confidence
            });
        }
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_available_languages() {
        let languages = WindowsOcr::available_languages();
        assert!(languages.is_ok());
        let langs = languages.unwrap();
        println!("Available OCR languages: {:?}", langs);
        // Most Windows installations have at least English
        assert!(!langs.is_empty());
    }

    #[test]
    fn test_create_engine() {
        let engine = WindowsOcr::new_default();
        assert!(engine.is_ok());
    }
}
