//! Storage Layer
//!
//! Handles persistence of profiles, settings, and logs using SQLite.

pub mod database;
pub mod profiles;

use anyhow::Result;
use std::path::PathBuf;

/// Get the application data directory
pub fn get_data_dir() -> Result<PathBuf> {
    let proj_dirs = directories::ProjectDirs::from("com", "gamerstoolkit", "GamersToolKit")
        .ok_or_else(|| anyhow::anyhow!("Could not determine data directory"))?;

    let data_dir = proj_dirs.data_dir().to_path_buf();
    std::fs::create_dir_all(&data_dir)?;

    Ok(data_dir)
}

/// Get the configuration directory
pub fn get_config_dir() -> Result<PathBuf> {
    let proj_dirs = directories::ProjectDirs::from("com", "gamerstoolkit", "GamersToolKit")
        .ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?;

    let config_dir = proj_dirs.config_dir().to_path_buf();
    std::fs::create_dir_all(&config_dir)?;

    Ok(config_dir)
}
