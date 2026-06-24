//! Commands module - CLI subcommand implementations.
//!
//! This module contains the implementations for all CLI subcommands,
//! organized by analysis type.

mod cc;
mod clones;
mod deps;
mod fix;
mod hal;
mod init;
mod mi;
mod raw;
mod stats;
pub(crate) mod utils;

// Re-export all public items
pub use cc::{run_cc, run_cc_with_tests, CcOptions};
pub use clones::{
    generate_clone_findings, generate_clone_findings_with_thresholds, run_clones, CloneOptions,
};
pub use deps::run_deps;
pub use fix::{run_fix_deadcode, DeadCodeFixOptions, FixResult};
pub use hal::{run_hal, run_hal_with_tests};
pub use init::{run_init, run_init_in};
pub use mi::{run_mi, run_mi_with_tests, MiOptions};
pub use raw::{run_raw, run_raw_with_tests};
#[allow(deprecated)]
pub use stats::run_stats;
pub use stats::{run_files, run_files_with_tests, run_stats_v2, Inspections, ScanOptions};
