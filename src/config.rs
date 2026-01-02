//! Configuration for arkai paths.
//!
//! Environment variables:
//! - ARKAI_HOME: Engine state (runs, catalog). Default: ~/.arkai
//! - ARKAI_LIBRARY: Content storage. Default: $ARKAI_HOME/library

use std::path::PathBuf;
use anyhow::{Context, Result};

/// Get the arkai home directory (engine state).
///
/// Checks ARKAI_HOME env var first, falls back to ~/.arkai
pub fn arkai_home() -> Result<PathBuf> {
    if let Ok(home) = std::env::var("ARKAI_HOME") {
        return Ok(PathBuf::from(home));
    }

    let home = dirs::home_dir().context("Failed to determine home directory")?;
    Ok(home.join(".arkai"))
}

/// Get the runs directory ($ARKAI_HOME/runs)
pub fn runs_dir() -> Result<PathBuf> {
    Ok(arkai_home()?.join("runs"))
}

/// Get the library directory.
///
/// Checks ARKAI_LIBRARY env var first, falls back to $ARKAI_HOME/library
pub fn library_dir() -> Result<PathBuf> {
    if let Ok(lib) = std::env::var("ARKAI_LIBRARY") {
        return Ok(PathBuf::from(lib));
    }
    Ok(arkai_home()?.join("library"))
}

/// Get the catalog path ($ARKAI_HOME/catalog.json)
pub fn catalog_path() -> Result<PathBuf> {
    Ok(arkai_home()?.join("catalog.json"))
}
