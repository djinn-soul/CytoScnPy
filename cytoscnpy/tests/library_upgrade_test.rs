//! Integration tests for verifying library upgrades.
//!
//! These tests ensure that upgraded dependencies work correctly with the codebase.
//! Run after updating dependencies in Cargo.toml to validate compatibility.

#![allow(clippy::unwrap_used)]

use cytoscnpy::config::Config;
use std::fs;
use std::path::Path;

// ============================================================================
// Helper Functions
// ============================================================================

fn setup_test_dir(path: &Path) {
    if path.exists() {
        fs::remove_dir_all(path).unwrap();
    }
    fs::create_dir(path).unwrap();
}

fn cleanup_test_dir(path: &Path) {
    if path.exists() {
        fs::remove_dir_all(path).unwrap();
    }
}

#[path = "library_upgrade_test/cases_1.rs"]
mod cases_1;
