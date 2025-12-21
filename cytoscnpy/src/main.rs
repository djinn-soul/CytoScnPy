//! Main binary entry point for the `CytoScnPy` static analysis tool.
//!
//! This binary simply delegates to the shared `entry_point::run_with_args()` function
//! to ensure consistent behavior across all entry points (CLI, Python bindings, etc.)

use anyhow::Result;

fn main() -> Result<()> {
    // Delegate CLI args to shared entry_point function (same as cytoscnpy-cli and Python)
    let code = cytoscnpy::entry_point::run_with_args(std::env::args().skip(1).collect())?;
    std::process::exit(code);
}
