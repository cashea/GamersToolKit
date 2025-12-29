//! Visual element detection module
//!
//! Template matching and pattern recognition for HUD elements, icons, etc.
//! Uses normalized cross-correlation for robust matching across different conditions.

use anyhow::{Context, Result};
use image::{GrayImage, Luma};
use std::collections::HashMap;
use std::path::Path;
use std::time::Instant;
use tracing::{debug, info, warn};

/// Template for matching visual elements
#[derive(Debug, Clone)]
pub struct Template {
    /// Template identifier
    pub id: String,
    /// Template image in grayscale
    grayscale: GrayImage,
    /// Original dimensions
    pub width: u32,
    pub height: u32,
    /// Minimum match threshold (0.0 - 1.0)
    pub threshold: f32,
    /// Optional mask for partial matching (white = match, black = ignore)
    mask: Option<GrayImage>,
    /// Pre-computed scales for multi-scale matching
    scales: Vec<f32>,
}

impl Template {
    /// Create a new template from RGBA image data
    pub fn from_rgba(id: &str, data: &[u8], width: u32, height: u32, threshold: f32) -> Result<Self> {
        let img = image::RgbaImage::from_raw(width, height, data.to_vec())
            .context("Failed to create image from RGBA data")?;

        let grayscale = image::DynamicImage::ImageRgba8(img).to_luma8();

        Ok(Self {
            id: id.to_string(),
            grayscale,
            width,
            height,
            threshold,
            mask: None,
            scales: vec![1.0], // Default: single scale
        })
    }

    /// Create a new template from BGRA image data (common for screen captures)
    pub fn from_bgra(id: &str, data: &[u8], width: u32, height: u32, threshold: f32) -> Result<Self> {
        // Convert BGRA to RGBA
        let mut rgba_data = data.to_vec();
        for chunk in rgba_data.chunks_mut(4) {
            chunk.swap(0, 2); // Swap B and R
        }

        Self::from_rgba(id, &rgba_data, width, height, threshold)
    }

    /// Load template from an image file
    pub fn from_file(id: &str, path: &Path, threshold: f32) -> Result<Self> {
        let img = image::open(path)
            .with_context(|| format!("Failed to load template image: {:?}", path))?;

        let grayscale = img.to_luma8();
        let (width, height) = grayscale.dimensions();

        Ok(Self {
            id: id.to_string(),
            grayscale,
            width,
            height,
            threshold,
            mask: None,
            scales: vec![1.0],
        })
    }

    /// Set scales for multi-scale matching
    pub fn with_scales(mut self, scales: Vec<f32>) -> Self {
        self.scales = scales;
        self
    }

    /// Set a mask for partial matching
    pub fn with_mask(mut self, mask: GrayImage) -> Self {
        self.mask = Some(mask);
        self
    }

    /// Get the grayscale image
    pub fn image(&self) -> &GrayImage {
        &self.grayscale
    }
}

/// Configuration for template matching
#[derive(Debug, Clone)]
pub struct MatcherConfig {
    /// Default threshold for matches (0.0 - 1.0)
    pub default_threshold: f32,
    /// Enable multi-scale matching
    pub multi_scale: bool,
    /// Scales to try for multi-scale matching
    pub scales: Vec<f32>,
    /// Maximum number of matches to return per template
    pub max_matches_per_template: usize,
    /// Minimum distance between matches (prevents duplicates)
    pub min_match_distance: u32,
    /// Enable match caching
    pub enable_cache: bool,
    /// Cache TTL in milliseconds
    pub cache_ttl_ms: u64,
}

impl Default for MatcherConfig {
    fn default() -> Self {
        Self {
            default_threshold: 0.8,
            multi_scale: false,
            scales: vec![0.8, 0.9, 1.0, 1.1, 1.2],
            max_matches_per_template: 10,
            min_match_distance: 10,
            enable_cache: true,
            cache_ttl_ms: 100, // 100ms cache
        }
    }
}

/// Cached match result
#[derive(Debug, Clone)]
struct CachedResult {
    matches: Vec<TemplateMatch>,
    timestamp: Instant,
}

/// Template matcher for detecting visual elements
pub struct TemplateMatcher {
    templates: HashMap<String, Template>,
    config: MatcherConfig,
    cache: HashMap<u64, CachedResult>,
}

impl TemplateMatcher {
    /// Create a new template matcher with default configuration
    pub fn new() -> Self {
        Self::with_config(MatcherConfig::default())
    }

    /// Create a new template matcher with custom configuration
    pub fn with_config(config: MatcherConfig) -> Self {
        Self {
            templates: HashMap::new(),
            config,
            cache: HashMap::new(),
        }
    }

    /// Add a template to match against
    pub fn add_template(&mut self, template: Template) {
        info!("Added template '{}' ({}x{})", template.id, template.width, template.height);
        self.templates.insert(template.id.clone(), template);
    }

    /// Remove a template by ID
    pub fn remove_template(&mut self, id: &str) -> Option<Template> {
        self.templates.remove(id)
    }

    /// Get a template by ID
    pub fn get_template(&self, id: &str) -> Option<&Template> {
        self.templates.get(id)
    }

    /// Get number of loaded templates
    pub fn template_count(&self) -> usize {
        self.templates.len()
    }

    /// Clear all templates
    pub fn clear_templates(&mut self) {
        self.templates.clear();
        self.cache.clear();
    }

    /// Find all template matches in a BGRA image
    pub fn find_matches(
        &mut self,
        image_data: &[u8],
        width: u32,
        height: u32,
    ) -> Result<Vec<TemplateMatch>> {
        if self.templates.is_empty() {
            return Ok(vec![]);
        }

        // Check cache
        let cache_key = self.compute_cache_key(image_data);
        if self.config.enable_cache {
            if let Some(cached) = self.cache.get(&cache_key) {
                if cached.timestamp.elapsed().as_millis() < self.config.cache_ttl_ms as u128 {
                    debug!("Using cached match results");
                    return Ok(cached.matches.clone());
                }
            }
        }

        let start = Instant::now();

        // Convert BGRA to grayscale
        let grayscale = bgra_to_grayscale(image_data, width, height);

        let mut all_matches = Vec::new();

        for template in self.templates.values() {
            let threshold = template.threshold.max(self.config.default_threshold);

            if self.config.multi_scale {
                // Multi-scale matching
                for &scale in &self.config.scales {
                    let matches = self.match_template_at_scale(&grayscale, template, scale, threshold)?;
                    all_matches.extend(matches);
                }
            } else {
                // Single-scale matching
                let matches = self.match_template(&grayscale, template, threshold)?;
                all_matches.extend(matches);
            }
        }

        // Remove duplicate matches
        all_matches = self.non_maximum_suppression(all_matches);

        debug!(
            "Template matching complete in {:?}: {} matches",
            start.elapsed(),
            all_matches.len()
        );

        // Cache results
        if self.config.enable_cache {
            self.cache.insert(cache_key, CachedResult {
                matches: all_matches.clone(),
                timestamp: Instant::now(),
            });
        }

        Ok(all_matches)
    }

    /// Match a single template against the image
    fn match_template(
        &self,
        image: &GrayImage,
        template: &Template,
        threshold: f32,
    ) -> Result<Vec<TemplateMatch>> {
        let template_img = template.image();
        let (img_w, img_h) = image.dimensions();
        let (tmpl_w, tmpl_h) = template_img.dimensions();

        if tmpl_w > img_w || tmpl_h > img_h {
            return Ok(vec![]);
        }

        let mut matches = Vec::new();

        // Normalized cross-correlation
        for y in 0..=(img_h - tmpl_h) {
            for x in 0..=(img_w - tmpl_w) {
                let score = normalized_cross_correlation(
                    image, template_img, x, y, template.mask.as_ref()
                );

                if score >= threshold {
                    matches.push(TemplateMatch {
                        template_id: template.id.clone(),
                        position: (x, y),
                        size: (tmpl_w, tmpl_h),
                        confidence: score,
                        scale: 1.0,
                    });
                }
            }
        }

        // Limit matches per template
        matches.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
        matches.truncate(self.config.max_matches_per_template);

        Ok(matches)
    }

    /// Match a template at a specific scale
    fn match_template_at_scale(
        &self,
        image: &GrayImage,
        template: &Template,
        scale: f32,
        threshold: f32,
    ) -> Result<Vec<TemplateMatch>> {
        if (scale - 1.0).abs() < 0.01 {
            return self.match_template(image, template, threshold);
        }

        // Scale the template
        let template_img = template.image();
        let (orig_w, orig_h) = template_img.dimensions();
        let new_w = ((orig_w as f32) * scale) as u32;
        let new_h = ((orig_h as f32) * scale) as u32;

        if new_w < 4 || new_h < 4 {
            return Ok(vec![]);
        }

        let scaled = image::imageops::resize(
            template_img,
            new_w,
            new_h,
            image::imageops::FilterType::Triangle,
        );

        let (img_w, img_h) = image.dimensions();
        if new_w > img_w || new_h > img_h {
            return Ok(vec![]);
        }

        let mut matches = Vec::new();

        for y in 0..=(img_h - new_h) {
            for x in 0..=(img_w - new_w) {
                let score = normalized_cross_correlation(image, &scaled, x, y, None);

                if score >= threshold {
                    matches.push(TemplateMatch {
                        template_id: template.id.clone(),
                        position: (x, y),
                        size: (new_w, new_h),
                        confidence: score,
                        scale,
                    });
                }
            }
        }

        matches.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
        matches.truncate(self.config.max_matches_per_template);

        Ok(matches)
    }

    /// Non-maximum suppression to remove duplicate matches
    fn non_maximum_suppression(&self, matches: Vec<TemplateMatch>) -> Vec<TemplateMatch> {
        if matches.is_empty() {
            return matches;
        }

        let mut result = Vec::new();
        let min_dist = self.config.min_match_distance;

        for m in matches {
            let dominated = result.iter().any(|existing: &TemplateMatch| {
                existing.template_id == m.template_id &&
                distance(existing.position, m.position) < min_dist &&
                existing.confidence >= m.confidence
            });

            if !dominated {
                // Remove any existing matches that this one dominates
                result.retain(|existing| {
                    !(existing.template_id == m.template_id &&
                      distance(existing.position, m.position) < min_dist &&
                      m.confidence > existing.confidence)
                });
                result.push(m);
            }
        }

        result
    }

    /// Compute cache key from image data (simple hash)
    fn compute_cache_key(&self, data: &[u8]) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        // Sample data for faster hashing
        let step = (data.len() / 1000).max(1);
        for (i, &byte) in data.iter().enumerate().step_by(step) {
            i.hash(&mut hasher);
            byte.hash(&mut hasher);
        }
        hasher.finish()
    }

    /// Clear the match cache
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }
}

impl Default for TemplateMatcher {
    fn default() -> Self {
        Self::new()
    }
}

/// A detected template match
#[derive(Debug, Clone)]
pub struct TemplateMatch {
    /// Template ID that matched
    pub template_id: String,
    /// Match location (x, y) - top-left corner
    pub position: (u32, u32),
    /// Match size (width, height)
    pub size: (u32, u32),
    /// Match confidence (0.0 - 1.0)
    pub confidence: f32,
    /// Scale at which match was found
    pub scale: f32,
}

impl TemplateMatch {
    /// Get the center point of the match
    pub fn center(&self) -> (u32, u32) {
        (
            self.position.0 + self.size.0 / 2,
            self.position.1 + self.size.1 / 2,
        )
    }

    /// Get bounding box as (x, y, width, height)
    pub fn bounds(&self) -> (u32, u32, u32, u32) {
        (self.position.0, self.position.1, self.size.0, self.size.1)
    }
}

/// Convert BGRA image data to grayscale
fn bgra_to_grayscale(data: &[u8], width: u32, height: u32) -> GrayImage {
    let mut gray = GrayImage::new(width, height);

    for y in 0..height {
        for x in 0..width {
            let idx = ((y * width + x) * 4) as usize;
            if idx + 2 < data.len() {
                let b = data[idx] as f32;
                let g = data[idx + 1] as f32;
                let r = data[idx + 2] as f32;
                // Standard grayscale conversion
                let gray_val = (0.299 * r + 0.587 * g + 0.114 * b) as u8;
                gray.put_pixel(x, y, Luma([gray_val]));
            }
        }
    }

    gray
}

/// Normalized cross-correlation between image region and template
fn normalized_cross_correlation(
    image: &GrayImage,
    template: &GrayImage,
    x: u32,
    y: u32,
    mask: Option<&GrayImage>,
) -> f32 {
    let (tmpl_w, tmpl_h) = template.dimensions();

    let mut sum_it = 0.0f64;  // Sum of image * template
    let mut sum_i2 = 0.0f64;  // Sum of image^2
    let mut sum_t2 = 0.0f64;  // Sum of template^2
    let mut sum_i = 0.0f64;   // Sum of image
    let mut sum_t = 0.0f64;   // Sum of template
    let mut count = 0.0f64;

    for ty in 0..tmpl_h {
        for tx in 0..tmpl_w {
            // Check mask
            if let Some(m) = mask {
                if m.get_pixel(tx, ty).0[0] < 128 {
                    continue;
                }
            }

            let img_val = image.get_pixel(x + tx, y + ty).0[0] as f64;
            let tmpl_val = template.get_pixel(tx, ty).0[0] as f64;

            sum_it += img_val * tmpl_val;
            sum_i2 += img_val * img_val;
            sum_t2 += tmpl_val * tmpl_val;
            sum_i += img_val;
            sum_t += tmpl_val;
            count += 1.0;
        }
    }

    if count == 0.0 {
        return 0.0;
    }

    // Zero-mean normalized cross-correlation
    let mean_i = sum_i / count;
    let mean_t = sum_t / count;

    let numerator = sum_it - count * mean_i * mean_t;
    let denom_i = (sum_i2 - count * mean_i * mean_i).sqrt();
    let denom_t = (sum_t2 - count * mean_t * mean_t).sqrt();

    let denominator = denom_i * denom_t;

    if denominator < 1e-10 {
        return 0.0;
    }

    (numerator / denominator).clamp(0.0, 1.0) as f32
}

/// Euclidean distance between two points
fn distance(a: (u32, u32), b: (u32, u32)) -> u32 {
    let dx = (a.0 as i32 - b.0 as i32).abs() as u32;
    let dy = (a.1 as i32 - b.1 as i32).abs() as u32;
    ((dx * dx + dy * dy) as f32).sqrt() as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_creation() {
        // Create a simple 2x2 template
        let data = vec![
            255, 255, 255, 255,  // White pixel (RGBA)
            0, 0, 0, 255,        // Black pixel
            0, 0, 0, 255,        // Black pixel
            255, 255, 255, 255,  // White pixel
        ];

        let template = Template::from_rgba("test", &data, 2, 2, 0.8);
        assert!(template.is_ok());

        let template = template.unwrap();
        assert_eq!(template.id, "test");
        assert_eq!(template.width, 2);
        assert_eq!(template.height, 2);
    }

    #[test]
    fn test_matcher_creation() {
        let matcher = TemplateMatcher::new();
        assert_eq!(matcher.template_count(), 0);
    }

    #[test]
    fn test_bgra_to_grayscale() {
        let data = vec![
            255, 0, 0, 255,    // Blue
            0, 255, 0, 255,    // Green
            0, 0, 255, 255,    // Red
            128, 128, 128, 255, // Gray
        ];

        let gray = bgra_to_grayscale(&data, 2, 2);

        // Check that blue pixel is dark, green is bright
        let blue_gray = gray.get_pixel(0, 0).0[0];
        let green_gray = gray.get_pixel(1, 0).0[0];

        assert!(green_gray > blue_gray, "Green should be brighter than blue in grayscale");
    }

    #[test]
    fn test_ncc_perfect_match() {
        let img_data = vec![
            100u8, 200, 100, 200,
            200, 100, 200, 100,
        ];
        let tmpl_data = vec![100u8, 200];

        let img = GrayImage::from_raw(4, 2, img_data).unwrap();
        let tmpl = GrayImage::from_raw(2, 1, tmpl_data).unwrap();

        let score = normalized_cross_correlation(&img, &tmpl, 0, 0, None);
        assert!(score > 0.99, "Perfect match should have high score: {}", score);
    }
}
