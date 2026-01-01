//! Event system for game state changes
//!
//! Detects and emits events when game state changes are detected.

use std::time::Instant;

/// Types of events that can be detected
#[derive(Debug, Clone)]
pub enum GameEvent {
    /// Text value changed in a region
    TextChanged {
        region_id: String,
        old_value: Option<String>,
        new_value: String,
    },
    /// Visual element appeared
    ElementAppeared {
        element_id: String,
    },
    /// Visual element disappeared
    ElementDisappeared {
        element_id: String,
    },
    /// Numeric value crossed threshold
    ThresholdCrossed {
        region_id: String,
        value: f64,
        threshold: f64,
        direction: ThresholdDirection,
    },
    /// Screen recognition changed to a new screen
    ScreenChanged {
        /// Previous screen ID (None if no screen was detected before)
        from_screen_id: Option<String>,
        /// Previous screen name
        from_screen_name: Option<String>,
        /// New screen ID
        to_screen_id: String,
        /// New screen name
        to_screen_name: String,
        /// Confidence level of the new screen detection
        confidence: f32,
    },
    /// No screen is currently being recognized (lost detection)
    ScreenLost {
        /// Previously detected screen ID
        previous_screen_id: String,
        /// Previously detected screen name
        previous_screen_name: String,
    },
}

/// Direction of threshold crossing
#[derive(Debug, Clone, Copy)]
pub enum ThresholdDirection {
    Above,
    Below,
}

/// A timestamped game event
#[derive(Debug)]
pub struct TimestampedEvent {
    /// The event
    pub event: GameEvent,
    /// When it occurred
    pub timestamp: Instant,
}

/// Event emitter for broadcasting game events
pub struct EventEmitter {
    // TODO: Event subscribers
}

impl EventEmitter {
    /// Create a new event emitter
    pub fn new() -> Self {
        Self {}
    }

    /// Emit an event to all subscribers
    pub fn emit(&self, _event: GameEvent) {
        // TODO: Notify subscribers
    }
}

impl Default for EventEmitter {
    fn default() -> Self {
        Self::new()
    }
}
