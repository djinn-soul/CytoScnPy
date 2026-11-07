//! View models for the report's Clones page.

use askama::Template;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
/// Represents a clone finding to be displayed in the report.
pub struct CloneItem {
    /// Similarity percentage (0-100).
    pub similarity: f64,
    /// Clone type (Type-1, Type-2, Type-3).
    pub clone_type: String,
    /// Name of the function/class/block.
    pub name: String,
    /// File where the clone is located.
    pub file: String,
    /// Line number of the clone.
    pub line: usize,
    /// Link to the file/line.
    pub link: String,
    /// Related file (original or other copy).
    pub related_file: String,
    /// Line number in related file.
    pub related_line: usize,
    /// Link to related file/line.
    pub related_link: String,
    /// Rule that reported the clone, e.g. `CSP-C001`.
    pub rule_id: String,
    /// Comment that suppresses this clone finding on its line.
    pub noqa: String,
}

/// View model for the Clones report page.
#[derive(Template)]
#[template(path = "clones.html")]
pub struct ClonesTemplate {
    /// List of clone findings.
    pub clones: Vec<CloneItem>,
    /// Generation timestamp.
    pub generated_at: String,
    /// CytoScnPy version.
    pub version: String,
    /// Root path for navigation (e.g. "." or "..")
    pub root_path: String,
}
