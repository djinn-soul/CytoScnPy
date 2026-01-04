//! Commands module - CLI subcommand implementations.
//!
//! This module contains the implementations for all CLI subcommands,
//! organized by analysis type.

mod cc;
mod clones;
mod fix;
mod hal;
mod mi;
mod raw;
mod stats;
mod utils;

// Re-export all public items
pub use cc::{run_cc, CcOptions};
pub use clones::{generate_clone_findings, run_clones, CloneOptions};
pub use fix::{run_fix_deadcode, DeadCodeFixOptions, FixResult};
pub use hal::run_hal;
pub use mi::{run_mi, MiOptions};
pub use raw::run_raw;
#[allow(deprecated)]
pub use stats::run_stats;
pub use stats::{run_files, run_stats_v2};
