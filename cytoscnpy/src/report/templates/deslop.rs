//! View models for the report's Deslop (Slop Index) page.

use askama::Template;
use serde::{Deserialize, Serialize};

/// One scored `DeSlopify` dimension, formatted for display.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeslopDimensionView {
    /// Dimension name, e.g. `Architecture clarity`.
    pub name: String,
    /// Weight of the dimension (all weights sum to 100).
    pub weight: u32,
    /// Slop rating from 0 (pristine) to 5 (maximum slop).
    pub rating: u32,
    /// Weighted contribution to the raw score, formatted.
    pub contribution: String,
    /// Explanation of the rating.
    pub evidence: String,
}

/// One remediation recommendation, formatted for display.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeslopRecommendationView {
    /// Short title.
    pub title: String,
    /// Dimension the recommendation targets.
    pub dimension: String,
    /// Estimated Slop Index reduction in points.
    pub estimated_reduction: u32,
    /// Estimated effort: `Low`, `Medium`, or `High`.
    pub effort: String,
    /// Problem explanation.
    pub description: String,
    /// Steps to address it.
    pub action_steps: Vec<String>,
    /// Files directly affected.
    pub affected_files: Vec<String>,
}

/// A failed `DeSlopify` gate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeslopGateView {
    /// Gate name, e.g. `max_slop_index`.
    pub check: String,
    /// Measured value.
    pub actual: String,
    /// Configured limit.
    pub limit: String,
}

/// The `DeSlopify` result shown on the report's Deslop page.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeslopView {
    /// Final Slop Index, 0 (clean) to 100 (disaster).
    pub slop_index: u32,
    /// Verdict band, e.g. `ACCEPTABLE`.
    pub verdict: String,
    /// Raw score before the size multiplier, formatted.
    pub raw_score: String,
    /// Size multiplier, formatted.
    pub size_multiplier: String,
    /// Whether every gate passed.
    pub passed_gate: bool,
    /// Scored dimensions.
    pub dimensions: Vec<DeslopDimensionView>,
    /// Remediation recommendations, highest impact first.
    pub recommendations: Vec<DeslopRecommendationView>,
    /// Gates that failed.
    pub gate_failures: Vec<DeslopGateView>,
    /// Python files discovered by the scan.
    pub files_discovered: usize,
    /// Python files the scan could check.
    pub files_checked: usize,
    /// Files skipped as `(path, reason)`.
    pub skipped_files: Vec<(String, String)>,
}

impl DeslopView {
    /// CSS color for the verdict, from clean (green) to disaster (red).
    #[must_use]
    pub fn verdict_color(&self) -> &'static str {
        match self.verdict.as_str() {
            "CLEAN" => "var(--grade-a)",
            "ACCEPTABLE" => "var(--grade-b)",
            "MESSY" => "var(--grade-c)",
            "SLOPPY" => "var(--grade-d)",
            _ => "var(--grade-f)",
        }
    }
}

/// View model for the Deslop page.
#[derive(Template)]
#[template(path = "deslop.html")]
pub struct DeslopTemplate {
    /// The `DeSlopify` result, or `None` for the page shown before
    /// `cytoscnpy deslop --html` has run.
    pub deslop: Option<DeslopView>,
    /// True when the page is written without the rest of the report, so the
    /// navigation shows only this page.
    pub standalone: bool,
    /// Generation timestamp.
    pub generated_at: String,
    /// CytoScnPy version.
    pub version: String,
    /// Root path for navigation (e.g. "." or "..")
    pub root_path: String,
    /// Theme this page opens in, overriding the report's default.
    pub theme: Option<&'static str>,
}
