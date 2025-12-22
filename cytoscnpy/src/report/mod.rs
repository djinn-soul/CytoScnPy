//! HTML report generation module.
//!
//! This module is gated behind the `html_report` feature flag and provides
//! functionality to generate static HTML reports from analysis results.


#[cfg(feature = "html_report")]
pub mod assets;
#[cfg(feature = "html_report")]
pub mod generator;
#[cfg(feature = "html_report")]
pub mod templates;

// Public API re-exports or stubs if needed when feature is disabled
#[cfg(not(feature = "html_report"))]
pub mod generator {
    // Stub or empty module
}
