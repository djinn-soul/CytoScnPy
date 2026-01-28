//! HTML report generation module.
//!
//! This module is gated behind the `html_report` feature flag and provides
//! functionality to generate static HTML reports from analysis results.

/// Default CSS and JS assets for the report.
#[cfg(feature = "html_report")]
pub mod assets;
/// Main report generation logic.
#[cfg(feature = "html_report")]
pub mod generator;
/// HTML templates for the report.
#[cfg(feature = "html_report")]
pub mod templates;

/// `GitHub` Annotations report generator.
pub mod github;
/// `GitLab` Code Quality report generator.
pub mod gitlab;
/// `JUnit` XML report generator.
pub mod junit;
/// Markdown report generator.
pub mod markdown;
/// SARIF report generator.
pub mod sarif;

// Public API re-exports or stubs if needed when feature is disabled
#[cfg(not(feature = "html_report"))]
pub mod generator {
    // Stub or empty module
}
