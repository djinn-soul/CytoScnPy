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

// Public API re-exports or stubs if needed when feature is disabled
#[cfg(not(feature = "html_report"))]
pub mod generator {
    // Stub or empty module
}
