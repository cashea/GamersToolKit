//! Screen Recognition Module
//!
//! Recognizes game screens (menus, HUDs, etc.) using template matching and anchor-based detection.
//! Supports hierarchical screen organization for context-aware OCR zone switching.

use anyhow::{Context, Result};
use image::{GrayImage, Luma};
use std::collections::HashMap;
use std::time::Instant;
use tracing::{debug, info};

use crate::storage::profiles::{
    AnchorType, ScreenAnchor, ScreenDefinition, ScreenMatchMode,
};

/// Result of screen recognition
#[derive(Debug, Clone)]
pub struct ScreenMatch {
    /// ID of the matched screen
    pub screen_id: String,
    /// Name of the matched screen
    pub screen_name: String,
    /// Overall match confidence (0.0-1.0)
    pub confidence: f32,
    /// Individual anchor matches
    pub matched_anchors: Vec<AnchorMatch>,
    /// Parent screen IDs in the hierarchy (from root to immediate parent)
    pub parent_chain: Vec<String>,
}

/// Result of matching a single anchor
#[derive(Debug, Clone)]
pub struct AnchorMatch {
    /// Anchor ID
    pub anchor_id: String,
    /// Whether this anchor matched
    pub matched: bool,
    /// Match confidence (0.0-1.0)
    pub confidence: f32,
    /// Detected text (for text anchors)
    pub detected_text: Option<String>,
}

/// A node in the screen hierarchy tree
#[derive(Debug, Clone)]
pub struct ScreenNode {
    /// Screen definition
    pub screen: ScreenDefinition,
    /// Child screens
    pub children: Vec<ScreenNode>,
    /// Depth in hierarchy (0 = root)
    pub depth: usize,
}

/// Configuration for screen recognition
#[derive(Debug, Clone)]
pub struct ScreenRecognitionConfig {
    /// Minimum confidence for a full screenshot match
    pub full_match_threshold: f32,
    /// Minimum confidence for individual anchor matches
    pub anchor_match_threshold: f32,
    /// Scale factor for full screenshot matching (lower = faster but less accurate)
    pub match_scale: f32,
    /// Enable caching of previous match to reduce redundant checks
    pub enable_cache: bool,
    /// Cache validity duration in milliseconds
    pub cache_ttl_ms: u64,
}

impl Default for ScreenRecognitionConfig {
    fn default() -> Self {
        Self {
            full_match_threshold: 0.7,
            anchor_match_threshold: 0.75,
            match_scale: 0.5, // Match at 50% resolution for speed
            enable_cache: true,
            cache_ttl_ms: 200,
        }
    }
}

/// Screen recognizer engine
pub struct ScreenRecognizer {
    /// Screen definitions indexed by ID
    screens: HashMap<String, ScreenDefinition>,
    /// Configuration
    config: ScreenRecognitionConfig,
    /// Cached templates for visual anchors (anchor_id -> grayscale image)
    anchor_templates: HashMap<String, GrayImage>,
    /// Cached full screen templates (screen_id -> grayscale image)
    screen_templates: HashMap<String, GrayImage>,
    /// Last match result for caching
    last_match: Option<(ScreenMatch, Instant)>,
    /// Pre-computed screen hierarchy
    hierarchy_cache: Option<Vec<ScreenNode>>,
}

impl ScreenRecognizer {
    /// Create a new screen recognizer
    pub fn new() -> Self {
        Self::with_config(ScreenRecognitionConfig::default())
    }

    /// Create a new screen recognizer with custom config
    pub fn with_config(config: ScreenRecognitionConfig) -> Self {
        Self {
            screens: HashMap::new(),
            config,
            anchor_templates: HashMap::new(),
            screen_templates: HashMap::new(),
            last_match: None,
            hierarchy_cache: None,
        }
    }

    /// Load screens from a list of screen definitions
    pub fn load_screens(&mut self, screens: Vec<ScreenDefinition>) {
        self.screens.clear();
        self.anchor_templates.clear();
        self.screen_templates.clear();
        self.hierarchy_cache = None;
        self.last_match = None;

        for screen in screens {
            self.add_screen(screen);
        }

        info!("Loaded {} screens for recognition", self.screens.len());
    }

    /// Add a screen definition
    pub fn add_screen(&mut self, screen: ScreenDefinition) {
        // Pre-process and cache visual anchor templates
        for anchor in &screen.anchors {
            if anchor.anchor_type == AnchorType::Visual {
                if let Some(ref data) = anchor.template_data {
                    if let Ok(template) = self.decode_template(data) {
                        self.anchor_templates.insert(anchor.id.clone(), template);
                    }
                }
            }
        }

        // Pre-process and cache full screen template
        if let Some(ref template) = screen.full_template {
            if let Ok(gray) = self.decode_template(&template.image_data) {
                self.screen_templates.insert(screen.id.clone(), gray);
            }
        }

        self.screens.insert(screen.id.clone(), screen);
        self.hierarchy_cache = None; // Invalidate hierarchy cache
    }

    /// Remove a screen by ID
    pub fn remove_screen(&mut self, id: &str) -> Option<ScreenDefinition> {
        // Remove cached templates
        if let Some(screen) = self.screens.get(id) {
            for anchor in &screen.anchors {
                self.anchor_templates.remove(&anchor.id);
            }
            self.screen_templates.remove(id);
        }

        self.hierarchy_cache = None;
        self.screens.remove(id)
    }

    /// Get the screen hierarchy as a tree
    pub fn get_hierarchy(&mut self) -> Vec<ScreenNode> {
        if let Some(ref cached) = self.hierarchy_cache {
            return cached.clone();
        }

        let hierarchy = self.build_hierarchy();
        self.hierarchy_cache = Some(hierarchy.clone());
        hierarchy
    }

    /// Build the screen hierarchy tree
    fn build_hierarchy(&self) -> Vec<ScreenNode> {
        let mut roots: Vec<ScreenNode> = Vec::new();

        // Find root screens (no parent)
        for screen in self.screens.values() {
            if screen.parent_id.is_none() {
                let node = self.build_node(screen, 0);
                roots.push(node);
            }
        }

        // Sort by priority (descending)
        roots.sort_by(|a, b| b.screen.priority.cmp(&a.screen.priority));
        roots
    }

    /// Recursively build a hierarchy node
    fn build_node(&self, screen: &ScreenDefinition, depth: usize) -> ScreenNode {
        let mut children: Vec<ScreenNode> = Vec::new();

        // Find child screens
        for child_screen in self.screens.values() {
            if child_screen.parent_id.as_ref() == Some(&screen.id) {
                children.push(self.build_node(child_screen, depth + 1));
            }
        }

        // Sort children by priority
        children.sort_by(|a, b| b.screen.priority.cmp(&a.screen.priority));

        ScreenNode {
            screen: screen.clone(),
            children,
            depth,
        }
    }

    /// Recognize the current screen from a captured frame
    ///
    /// # Arguments
    /// * `image_data` - BGRA image data
    /// * `width` - Image width
    /// * `height` - Image height
    /// * `ocr_fn` - Optional function to get OCR text for a region (for text anchors)
    pub fn recognize<F>(
        &mut self,
        image_data: &[u8],
        width: u32,
        height: u32,
        ocr_fn: Option<F>,
    ) -> Option<ScreenMatch>
    where
        F: Fn(u32, u32, u32, u32) -> Option<String>,
    {
        // Check cache first
        if self.config.enable_cache {
            if let Some((ref cached_match, timestamp)) = self.last_match {
                if timestamp.elapsed().as_millis() < self.config.cache_ttl_ms as u128 {
                    debug!("Using cached screen match: {}", cached_match.screen_name);
                    return Some(cached_match.clone());
                }
            }
        }

        let start = Instant::now();

        // Convert to grayscale for matching
        let grayscale = bgra_to_grayscale(image_data, width, height);

        // Get sorted screens by priority
        let mut screens: Vec<_> = self.screens.values().collect();
        screens.sort_by(|a, b| b.priority.cmp(&a.priority));

        let mut best_match: Option<ScreenMatch> = None;
        let mut best_confidence: f32 = 0.0;

        // Track which screens matched (for hierarchy validation)
        let mut matched_screen_ids: std::collections::HashSet<String> = std::collections::HashSet::new();

        for screen in screens {
            if !screen.enabled {
                continue;
            }

            // Check hierarchy constraint
            if let Some(ref parent_id) = screen.parent_id {
                if !matched_screen_ids.contains(parent_id) {
                    // Parent didn't match, skip this screen
                    continue;
                }
            }

            // Try to match this screen
            if let Some(screen_match) = self.match_screen(screen, &grayscale, width, height, &ocr_fn) {
                if screen_match.confidence >= screen.match_threshold {
                    matched_screen_ids.insert(screen.id.clone());

                    // Keep track of best match
                    if screen_match.confidence > best_confidence {
                        best_confidence = screen_match.confidence;
                        best_match = Some(screen_match);
                    }
                }
            }
        }

        debug!(
            "Screen recognition completed in {:?}: {:?}",
            start.elapsed(),
            best_match.as_ref().map(|m| &m.screen_name)
        );

        // Update cache
        if self.config.enable_cache {
            self.last_match = best_match.clone().map(|m| (m, Instant::now()));
        }

        best_match
    }

    /// Match a single screen against the image
    fn match_screen<F>(
        &self,
        screen: &ScreenDefinition,
        grayscale: &GrayImage,
        width: u32,
        height: u32,
        ocr_fn: &Option<F>,
    ) -> Option<ScreenMatch>
    where
        F: Fn(u32, u32, u32, u32) -> Option<String>,
    {
        match screen.match_mode {
            ScreenMatchMode::FullScreenshot => {
                self.match_full_screenshot(screen, grayscale)
            }
            ScreenMatchMode::Anchors => {
                self.match_anchors(screen, grayscale, width, height, ocr_fn)
            }
        }
    }

    /// Match using full screenshot template
    fn match_full_screenshot(
        &self,
        screen: &ScreenDefinition,
        grayscale: &GrayImage,
    ) -> Option<ScreenMatch> {
        let template = self.screen_templates.get(&screen.id)?;

        // Scale down for faster matching
        let scaled_image = if (self.config.match_scale - 1.0).abs() > 0.01 {
            let new_w = (grayscale.width() as f32 * self.config.match_scale) as u32;
            let new_h = (grayscale.height() as f32 * self.config.match_scale) as u32;
            image::imageops::resize(grayscale, new_w, new_h, image::imageops::FilterType::Triangle)
        } else {
            grayscale.clone()
        };

        let scaled_template = if (self.config.match_scale - 1.0).abs() > 0.01 {
            let new_w = (template.width() as f32 * self.config.match_scale) as u32;
            let new_h = (template.height() as f32 * self.config.match_scale) as u32;
            image::imageops::resize(template, new_w, new_h, image::imageops::FilterType::Triangle)
        } else {
            template.clone()
        };

        // Compute normalized cross-correlation at center position
        // For full screenshot matching, we compare the entire image
        let confidence = compute_image_similarity(&scaled_image, &scaled_template);

        if confidence >= self.config.full_match_threshold {
            Some(ScreenMatch {
                screen_id: screen.id.clone(),
                screen_name: screen.name.clone(),
                confidence,
                matched_anchors: vec![],
                parent_chain: self.get_parent_chain(&screen.id),
            })
        } else {
            None
        }
    }

    /// Match using anchor regions
    fn match_anchors<F>(
        &self,
        screen: &ScreenDefinition,
        grayscale: &GrayImage,
        width: u32,
        height: u32,
        ocr_fn: &Option<F>,
    ) -> Option<ScreenMatch>
    where
        F: Fn(u32, u32, u32, u32) -> Option<String>,
    {
        if screen.anchors.is_empty() {
            return None;
        }

        let mut matched_anchors = Vec::new();
        let mut required_match_count = 0;
        let mut required_total = 0;
        let mut total_confidence = 0.0;
        let mut anchor_count = 0;

        for anchor in &screen.anchors {
            let anchor_match = self.match_anchor(anchor, grayscale, width, height, ocr_fn);

            if anchor.required {
                required_total += 1;
                if anchor_match.matched {
                    required_match_count += 1;
                }
            }

            if anchor_match.matched {
                total_confidence += anchor_match.confidence;
                anchor_count += 1;
            }

            matched_anchors.push(anchor_match);
        }

        // All required anchors must match
        if required_match_count < required_total {
            return None;
        }

        // Calculate overall confidence
        let confidence = if anchor_count > 0 {
            total_confidence / anchor_count as f32
        } else {
            0.0
        };

        Some(ScreenMatch {
            screen_id: screen.id.clone(),
            screen_name: screen.name.clone(),
            confidence,
            matched_anchors,
            parent_chain: self.get_parent_chain(&screen.id),
        })
    }

    /// Match a single anchor
    fn match_anchor<F>(
        &self,
        anchor: &ScreenAnchor,
        grayscale: &GrayImage,
        width: u32,
        height: u32,
        ocr_fn: &Option<F>,
    ) -> AnchorMatch
    where
        F: Fn(u32, u32, u32, u32) -> Option<String>,
    {
        // Convert normalized bounds to pixel coordinates
        let x = (anchor.bounds.0 * width as f32) as u32;
        let y = (anchor.bounds.1 * height as f32) as u32;
        let w = (anchor.bounds.2 * width as f32) as u32;
        let h = (anchor.bounds.3 * height as f32) as u32;

        match anchor.anchor_type {
            AnchorType::Visual => {
                self.match_visual_anchor(anchor, grayscale, x, y, w, h)
            }
            AnchorType::Text => {
                self.match_text_anchor(anchor, x, y, w, h, ocr_fn)
            }
        }
    }

    /// Match a visual anchor using template matching
    fn match_visual_anchor(
        &self,
        anchor: &ScreenAnchor,
        grayscale: &GrayImage,
        x: u32,
        y: u32,
        w: u32,
        h: u32,
    ) -> AnchorMatch {
        let template = match self.anchor_templates.get(&anchor.id) {
            Some(t) => t,
            None => {
                return AnchorMatch {
                    anchor_id: anchor.id.clone(),
                    matched: false,
                    confidence: 0.0,
                    detected_text: None,
                };
            }
        };

        // Extract the region from the grayscale image
        let region = extract_gray_region(grayscale, x, y, w, h);

        // Compute similarity
        let confidence = compute_image_similarity(&region, template);

        AnchorMatch {
            anchor_id: anchor.id.clone(),
            matched: confidence >= self.config.anchor_match_threshold,
            confidence,
            detected_text: None,
        }
    }

    /// Match a text anchor using OCR
    fn match_text_anchor<F>(
        &self,
        anchor: &ScreenAnchor,
        x: u32,
        y: u32,
        w: u32,
        h: u32,
        ocr_fn: &Option<F>,
    ) -> AnchorMatch
    where
        F: Fn(u32, u32, u32, u32) -> Option<String>,
    {
        let expected_text = match &anchor.expected_text {
            Some(t) => t,
            None => {
                return AnchorMatch {
                    anchor_id: anchor.id.clone(),
                    matched: false,
                    confidence: 0.0,
                    detected_text: None,
                };
            }
        };

        // Get OCR result for the region
        let detected_text = match ocr_fn {
            Some(f) => f(x, y, w, h),
            None => None,
        };

        let (matched, confidence) = match &detected_text {
            Some(text) => {
                let similarity = text_similarity(text, expected_text);
                (similarity >= anchor.text_similarity, similarity)
            }
            None => (false, 0.0),
        };

        AnchorMatch {
            anchor_id: anchor.id.clone(),
            matched,
            confidence,
            detected_text,
        }
    }

    /// Get the parent chain for a screen ID
    fn get_parent_chain(&self, screen_id: &str) -> Vec<String> {
        let mut chain = Vec::new();
        let mut current_id = screen_id;

        while let Some(screen) = self.screens.get(current_id) {
            if let Some(ref parent_id) = screen.parent_id {
                chain.insert(0, parent_id.clone());
                current_id = parent_id;
            } else {
                break;
            }
        }

        chain
    }

    /// Decode PNG template data to grayscale image
    fn decode_template(&self, data: &[u8]) -> Result<GrayImage> {
        let img = image::load_from_memory(data)
            .context("Failed to decode template image")?;
        Ok(img.to_luma8())
    }

    /// Clear all cached data
    pub fn clear_cache(&mut self) {
        self.last_match = None;
    }

    /// Get the number of loaded screens
    pub fn screen_count(&self) -> usize {
        self.screens.len()
    }

    /// Get a screen by ID
    pub fn get_screen(&self, id: &str) -> Option<&ScreenDefinition> {
        self.screens.get(id)
    }
}

impl Default for ScreenRecognizer {
    fn default() -> Self {
        Self::new()
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

/// Extract a region from a grayscale image
fn extract_gray_region(image: &GrayImage, x: u32, y: u32, w: u32, h: u32) -> GrayImage {
    let (img_w, img_h) = image.dimensions();

    // Clamp to image bounds
    let x = x.min(img_w);
    let y = y.min(img_h);
    let w = w.min(img_w.saturating_sub(x));
    let h = h.min(img_h.saturating_sub(y));

    if w == 0 || h == 0 {
        return GrayImage::new(1, 1);
    }

    let mut region = GrayImage::new(w, h);

    for ry in 0..h {
        for rx in 0..w {
            let pixel = image.get_pixel(x + rx, y + ry);
            region.put_pixel(rx, ry, *pixel);
        }
    }

    region
}

/// Compute similarity between two grayscale images using normalized cross-correlation
fn compute_image_similarity(image: &GrayImage, template: &GrayImage) -> f32 {
    let (img_w, img_h) = image.dimensions();
    let (tmpl_w, tmpl_h) = template.dimensions();

    // If sizes differ significantly, resize template to match
    let template = if (img_w as i32 - tmpl_w as i32).abs() > 5 || (img_h as i32 - tmpl_h as i32).abs() > 5 {
        image::imageops::resize(template, img_w, img_h, image::imageops::FilterType::Triangle)
    } else {
        template.clone()
    };

    let (tmpl_w, tmpl_h) = template.dimensions();
    let compare_w = img_w.min(tmpl_w);
    let compare_h = img_h.min(tmpl_h);

    if compare_w == 0 || compare_h == 0 {
        return 0.0;
    }

    // Compute normalized cross-correlation
    let mut sum_it = 0.0f64;
    let mut sum_i2 = 0.0f64;
    let mut sum_t2 = 0.0f64;
    let mut sum_i = 0.0f64;
    let mut sum_t = 0.0f64;
    let count = (compare_w * compare_h) as f64;

    for y in 0..compare_h {
        for x in 0..compare_w {
            let img_val = image.get_pixel(x, y).0[0] as f64;
            let tmpl_val = template.get_pixel(x, y).0[0] as f64;

            sum_it += img_val * tmpl_val;
            sum_i2 += img_val * img_val;
            sum_t2 += tmpl_val * tmpl_val;
            sum_i += img_val;
            sum_t += tmpl_val;
        }
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

/// Compute text similarity using Levenshtein distance
fn text_similarity(a: &str, b: &str) -> f32 {
    let a = a.to_lowercase();
    let b = b.to_lowercase();

    if a.is_empty() && b.is_empty() {
        return 1.0;
    }

    if a.is_empty() || b.is_empty() {
        return 0.0;
    }

    // Use strsim crate if available, otherwise simple algorithm
    let distance = levenshtein_distance(&a, &b);
    let max_len = a.len().max(b.len()) as f32;

    1.0 - (distance as f32 / max_len)
}

/// Simple Levenshtein distance implementation
fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();

    let len_a = a_chars.len();
    let len_b = b_chars.len();

    if len_a == 0 { return len_b; }
    if len_b == 0 { return len_a; }

    let mut matrix = vec![vec![0; len_b + 1]; len_a + 1];

    for i in 0..=len_a { matrix[i][0] = i; }
    for j in 0..=len_b { matrix[0][j] = j; }

    for i in 1..=len_a {
        for j in 1..=len_b {
            let cost = if a_chars[i - 1] == b_chars[j - 1] { 0 } else { 1 };
            matrix[i][j] = (matrix[i - 1][j] + 1)
                .min(matrix[i][j - 1] + 1)
                .min(matrix[i - 1][j - 1] + cost);
        }
    }

    matrix[len_a][len_b]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_similarity() {
        assert!((text_similarity("hello", "hello") - 1.0).abs() < 0.001);
        assert!((text_similarity("hello", "hallo") - 0.8).abs() < 0.001);
        assert!((text_similarity("HELLO", "hello") - 1.0).abs() < 0.001);
        assert!(text_similarity("abc", "xyz") < 0.5);
    }

    #[test]
    fn test_levenshtein_distance() {
        assert_eq!(levenshtein_distance("", ""), 0);
        assert_eq!(levenshtein_distance("abc", ""), 3);
        assert_eq!(levenshtein_distance("", "abc"), 3);
        assert_eq!(levenshtein_distance("kitten", "sitting"), 3);
    }

    #[test]
    fn test_screen_recognizer_creation() {
        let recognizer = ScreenRecognizer::new();
        assert_eq!(recognizer.screen_count(), 0);
    }

    #[test]
    fn test_add_remove_screen() {
        let mut recognizer = ScreenRecognizer::new();

        let screen = ScreenDefinition {
            id: "test".to_string(),
            name: "Test Screen".to_string(),
            parent_id: None,
            match_mode: ScreenMatchMode::Anchors,
            anchors: vec![],
            full_template: None,
            match_threshold: 0.8,
            enabled: true,
            priority: 10,
            ocr_zone_overrides: vec![],
            rules_to_trigger: vec![],
            show_notification: true,
        };

        recognizer.add_screen(screen);
        assert_eq!(recognizer.screen_count(), 1);
        assert!(recognizer.get_screen("test").is_some());

        recognizer.remove_screen("test");
        assert_eq!(recognizer.screen_count(), 0);
    }

    #[test]
    fn test_hierarchy() {
        let mut recognizer = ScreenRecognizer::new();

        // Add parent screen
        recognizer.add_screen(ScreenDefinition {
            id: "parent".to_string(),
            name: "Parent".to_string(),
            parent_id: None,
            match_mode: ScreenMatchMode::Anchors,
            anchors: vec![],
            full_template: None,
            match_threshold: 0.8,
            enabled: true,
            priority: 10,
            ocr_zone_overrides: vec![],
            rules_to_trigger: vec![],
            show_notification: true,
        });

        // Add child screen
        recognizer.add_screen(ScreenDefinition {
            id: "child".to_string(),
            name: "Child".to_string(),
            parent_id: Some("parent".to_string()),
            match_mode: ScreenMatchMode::Anchors,
            anchors: vec![],
            full_template: None,
            match_threshold: 0.8,
            enabled: true,
            priority: 5,
            ocr_zone_overrides: vec![],
            rules_to_trigger: vec![],
            show_notification: true,
        });

        let hierarchy = recognizer.get_hierarchy();
        assert_eq!(hierarchy.len(), 1); // One root
        assert_eq!(hierarchy[0].screen.id, "parent");
        assert_eq!(hierarchy[0].children.len(), 1);
        assert_eq!(hierarchy[0].children[0].screen.id, "child");
    }

    #[test]
    fn test_image_similarity_identical() {
        let img = GrayImage::from_fn(10, 10, |x, y| {
            Luma([((x + y) % 256) as u8])
        });

        let similarity = compute_image_similarity(&img, &img);
        assert!((similarity - 1.0).abs() < 0.01, "Identical images should have similarity ~1.0");
    }

    #[test]
    fn test_bgra_to_grayscale() {
        let data = vec![
            255, 0, 0, 255,   // Blue
            0, 255, 0, 255,   // Green
            0, 0, 255, 255,   // Red
            128, 128, 128, 255, // Gray
        ];

        let gray = bgra_to_grayscale(&data, 2, 2);
        assert_eq!(gray.dimensions(), (2, 2));

        // Green should be brighter than blue in grayscale
        let blue_val = gray.get_pixel(0, 0).0[0];
        let green_val = gray.get_pixel(1, 0).0[0];
        assert!(green_val > blue_val);
    }
}
