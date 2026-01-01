//! Rules engine using rhai scripting
//!
//! Allows game profiles to define custom logic for generating tips and alerts.

use anyhow::Result;

/// A rule definition from a game profile
#[derive(Debug, Clone)]
pub struct Rule {
    /// Rule identifier
    pub id: String,
    /// Rule name for display
    pub name: String,
    /// Whether this rule is enabled
    pub enabled: bool,
    /// Rhai script code
    pub script: String,
}

/// Rules engine powered by rhai
pub struct RulesEngine {
    // TODO: rhai::Engine
}

impl RulesEngine {
    /// Create a new rules engine
    pub fn new() -> Result<Self> {
        // TODO: Initialize rhai engine with custom functions
        Ok(Self {})
    }

    /// Register a rule
    pub fn register_rule(&mut self, _rule: Rule) -> Result<()> {
        // TODO: Compile and register rule script
        Ok(())
    }

    /// Evaluate all rules against current game state
    pub fn evaluate(&self, _game_state: &GameState) -> Result<Vec<RuleResult>> {
        // TODO: Run all enabled rules
        Ok(vec![])
    }
}

impl Default for RulesEngine {
    fn default() -> Self {
        Self::new().expect("Failed to create rules engine")
    }
}

/// Current game state derived from vision analysis
#[derive(Debug, Default)]
pub struct GameState {
    /// Detected text by region ID
    pub text_values: std::collections::HashMap<String, String>,
    /// Detected elements by ID
    pub elements: std::collections::HashMap<String, bool>,
    /// Current screen recognition context
    pub screen_context: ScreenContext,
}

/// Screen recognition context for rules
#[derive(Debug, Clone, Default)]
pub struct ScreenContext {
    /// Currently detected screen ID (None if no screen detected)
    pub current_screen_id: Option<String>,
    /// Currently detected screen name
    pub current_screen_name: Option<String>,
    /// Detection confidence (0.0 - 1.0)
    pub confidence: f32,
    /// Whether screen just changed this frame
    pub just_changed: bool,
    /// Previous screen ID (if screen just changed)
    pub previous_screen_id: Option<String>,
    /// Previous screen name (if screen just changed)
    pub previous_screen_name: Option<String>,
    /// Parent screen chain (from root to current, for hierarchical screens)
    pub parent_chain: Vec<String>,
}

impl ScreenContext {
    /// Create a new screen context from a screen match
    pub fn from_match(
        screen_id: Option<String>,
        screen_name: Option<String>,
        confidence: f32,
        just_changed: bool,
        previous_id: Option<String>,
        previous_name: Option<String>,
        parent_chain: Vec<String>,
    ) -> Self {
        Self {
            current_screen_id: screen_id,
            current_screen_name: screen_name,
            confidence,
            just_changed,
            previous_screen_id: previous_id,
            previous_screen_name: previous_name,
            parent_chain,
        }
    }

    /// Check if the current screen matches a given name (case-insensitive)
    pub fn screen_is(&self, name: &str) -> bool {
        self.current_screen_name
            .as_ref()
            .map(|n| n.eq_ignore_ascii_case(name))
            .unwrap_or(false)
    }

    /// Check if the current screen is a child of a screen with the given name
    pub fn is_child_of(&self, parent_name: &str) -> bool {
        self.parent_chain
            .iter()
            .any(|p| p.eq_ignore_ascii_case(parent_name))
    }

    /// Check if any screen is currently detected
    pub fn has_screen(&self) -> bool {
        self.current_screen_id.is_some()
    }
}

/// Result of evaluating a rule
#[derive(Debug)]
pub struct RuleResult {
    /// Rule that triggered
    pub rule_id: String,
    /// Generated tip message
    pub message: Option<String>,
    /// Whether to trigger alert
    pub alert: bool,
}
