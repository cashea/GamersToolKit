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
mod shared;
mod dashboard;
mod app;

use anyhow::Result;
use clap::Parser;
use parking_lot::RwLock;
use std::sync::Arc;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

use crate::analysis::Tip;
use crate::config::AppConfig;
use crate::overlay::{list_monitors, OverlayConfig, OverlayManager};
use crate::shared::SharedAppState;

/// GamersToolKit - Real-time game analysis overlay
#[derive(Parser, Debug)]
#[command(name = "gamers-toolkit")]
#[command(about = "A read-only screen parsing tool for contextual gameplay tips")]
struct Args {
    /// Monitor index to display overlay on (0 = primary)
    #[arg(short, long, default_value = "0")]
    monitor: usize,

    /// List available monitors and exit
    #[arg(long)]
    list_monitors: bool,

    /// Run in dashboard mode (default)
    #[arg(long, default_value = "true")]
    dashboard: bool,

    /// Run in overlay-only mode (no dashboard window)
    #[arg(long)]
    overlay_only: bool,
}

fn main() -> Result<()> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    let args = Args::parse();

    // List monitors mode
    if args.list_monitors {
        println!("Available monitors:");
        let monitors = list_monitors();
        if monitors.is_empty() {
            println!("  No monitors detected (GLFW initialization may have failed)");
        } else {
            for monitor in &monitors {
                println!(
                    "  [{}] {} - {}x{} at ({}, {}){}",
                    monitor.index,
                    monitor.name.as_deref().unwrap_or("Unknown"),
                    monitor.work_area.2,
                    monitor.work_area.3,
                    monitor.work_area.0,
                    monitor.work_area.1,
                    if monitor.is_primary { " (primary)" } else { "" }
                );
            }
        }
        return Ok(());
    }

    info!("GamersToolKit starting...");
    info!("Read-only mode: Screen capture and analysis only");
    info!("No game memory access, no input injection");

    // Load or create configuration
    let config = load_or_create_config();

    // Create shared state
    let shared_state = Arc::new(RwLock::new(SharedAppState::new(config)));

    if args.overlay_only {
        // Run in overlay-only mode
        run_overlay_only(args.monitor, shared_state)?;
    } else {
        // Run in dashboard mode (default)
        run_with_dashboard(args.monitor, shared_state)?;
    }

    info!("GamersToolKit shutdown complete");

    Ok(())
}

/// Load configuration from file or create default
fn load_or_create_config() -> AppConfig {
    if let Ok(config_dir) = storage::get_config_dir() {
        let config_path = config_dir.join("config.toml");
        if config_path.exists() {
            if let Ok(config) = config::load_config(&config_path) {
                info!("Loaded configuration from {:?}", config_path);
                return config;
            }
        }
    }
    info!("Using default configuration");
    AppConfig::default()
}

/// Run in overlay-only mode
fn run_overlay_only(monitor: usize, shared_state: Arc<RwLock<SharedAppState>>) -> Result<()> {
    info!("Running in overlay-only mode");

    // Configure overlay for selected monitor
    let mut config = {
        let state = shared_state.read();
        state.overlay_config.clone()
    };
    config.monitor_index = Some(monitor);

    info!("Overlay will appear on monitor {}", monitor);

    // Create overlay manager
    let manager = OverlayManager::new(config)?;

    // Update runtime state
    {
        let mut state = shared_state.write();
        state.runtime.is_overlay_running = true;
        state.runtime.overlay_visible = true;
    }

    // Send a demo tip after a short delay
    let tip_sender = manager.tip_sender();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(2));
        let _ = tip_sender.send(Tip {
            id: "demo_1".to_string(),
            message: "GamersToolKit overlay is working!".to_string(),
            priority: 50,
            duration_ms: Some(5000),
            play_sound: false,
        });

        std::thread::sleep(std::time::Duration::from_secs(3));
        let _ = tip_sender.send(Tip {
            id: "demo_2".to_string(),
            message: "Press ESC or close the window to exit".to_string(),
            priority: 25,
            duration_ms: Some(8000),
            play_sound: false,
        });
    });

    info!("Starting overlay... (Press ESC to exit)");

    // Run the overlay (blocking)
    manager.run()?;

    Ok(())
}

/// Run with dashboard window
fn run_with_dashboard(monitor: usize, shared_state: Arc<RwLock<SharedAppState>>) -> Result<()> {
    info!("Running in dashboard mode");

    // Update overlay config with monitor
    {
        let mut state = shared_state.write();
        state.overlay_config.monitor_index = Some(monitor);
    }

    // Run the dashboard (blocking)
    if let Err(e) = dashboard::app::run_dashboard(shared_state) {
        tracing::error!("Dashboard error: {}", e);
    }

    Ok(())
}
