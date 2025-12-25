//! GamersToolKit - Real-time game analysis and assistance overlay
//!
//! A read-only screen parsing tool that provides contextual gameplay tips
//! without interacting with game memory or inputs.

mod capture;
mod vision;
mod analysis;
mod overlay;
mod storage;
mod config;

use anyhow::Result;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

fn main() -> Result<()> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("GamersToolKit starting...");
    info!("Read-only mode: Screen capture and analysis only");
    info!("No game memory access, no input injection");

    // TODO: Initialize components
    // 1. Load configuration
    // 2. Initialize screen capture
    // 3. Start OCR/vision pipeline
    // 4. Load game profiles and rules
    // 5. Launch overlay window

    info!("GamersToolKit initialized successfully");

    Ok(())
}
