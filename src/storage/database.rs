//! SQLite database for persistent storage

use anyhow::Result;
use std::path::Path;

/// Database connection wrapper
pub struct Database {
    // TODO: rusqlite::Connection
}

impl Database {
    /// Open or create database at path
    pub fn open(_path: &Path) -> Result<Self> {
        // TODO: Open SQLite database
        Ok(Self {})
    }

    /// Initialize database schema
    pub fn init_schema(&self) -> Result<()> {
        // TODO: Create tables if not exist
        // - settings
        // - profiles
        // - logs
        Ok(())
    }
}
