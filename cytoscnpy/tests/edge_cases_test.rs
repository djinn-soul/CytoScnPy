//! Comprehensive Edge Case Tests for Rust `CytoScnPy` Implementation
#![allow(clippy::expect_used)]

use std::fs::{self, File};
use std::io::Write;
use tempfile::TempDir;

/// Helper struct to create temporary Python projects for testing
struct TestProject {
    root: TempDir,
}

impl TestProject {
    /// Create a new temporary test project
    fn new() -> Self {
        TestProject {
            root: TempDir::new().expect("Failed to create temp directory"),
        }
    }

    /// Get the root path of the test project
    fn path(&self) -> &std::path::Path {
        self.root.path()
    }

    /// Create a Python file in the project
    fn create_file(&self, name: &str, content: &str) -> std::io::Result<()> {
        let file_path = self.root.path().join(name);
        let mut file = File::create(file_path)?;
        file.write_all(content.as_bytes())?;
        Ok(())
    }

    /// Create a subdirectory and optionally an __init__.py file for a package
    fn create_package(&self, name: &str) -> std::io::Result<()> {
        let package_path = self.root.path().join(name);
        fs::create_dir_all(&package_path)?;
        let init_path = package_path.join("__init__.py");
        File::create(init_path)?.write_all(b"")?;
        Ok(())
    }
}

#[path = "edge_cases_test/cases_1.rs"]
mod cases_1;
#[path = "edge_cases_test/cases_2.rs"]
mod cases_2;
#[path = "edge_cases_test/cases_3.rs"]
mod cases_3;
#[path = "edge_cases_test/cases_4.rs"]
mod cases_4;
#[path = "edge_cases_test/cases_5.rs"]
mod cases_5;
