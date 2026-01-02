//! Configuration for arkai paths.
//!
//! Supports ARKAI_HOME environment variable to customize storage location.
//! Default: ~/.arkai

use std::path::PathBuf;
use anyhow::{Context, Result};

/// Get the arkai home directory.
///
/// Checks ARKAI_HOME env var first, falls back to ~/.arkai
pub fn arkai_home() -> Result<PathBuf> {
    if let Ok(home) = std::env::var("ARKAI_HOME") {
        return Ok(PathBuf::from(home));
    }

    let home = dirs::home_dir().context("Failed to determine home directory")?;
    Ok(home.join(".arkai"))
}

/// Get the runs directory (~/.arkai/runs or $ARKAI_HOME/runs)
pub fn runs_dir() -> Result<PathBuf> {
    Ok(arkai_home()?.join("runs"))
}

/// Get the library directory (~/.arkai/library or $ARKAI_HOME/library)
pub fn library_dir() -> Result<PathBuf> {
    Ok(arkai_home()?.join("library"))
}

/// Get the catalog path (~/.arkai/catalog.json or $ARKAI_HOME/catalog.json)
pub fn catalog_path() -> Result<PathBuf> {
    Ok(arkai_home()?.join("catalog.json"))
}
