//! Command-line interface entry point for `CytoScnPy`.

use anyhow::Result;
use cytoscnpy::entry_point;
fn main() -> Result<()> {
    // Delegate CLI args to shared entry_point function
    let code = entry_point::run_with_args(std::env::args().skip(1).collect())?;
    std::process::exit(code);
}
