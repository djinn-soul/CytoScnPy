//! Heap profiling example using dhat.
//!
//! Run with:
//! ```bash
//! cargo run --example heap_profile --features heap-profile -- <path-to-analyze>
//! ```
//!
//! This will output a `dhat-heap.json` file that can be viewed at:
//! <https://nnethercote.github.io/dh_view/dh_view.html>

#[cfg(feature = "heap-profile")]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

use cytoscnpy::analyzer::CytoScnPy;
use std::path::PathBuf;

fn main() {
    #[cfg(feature = "heap-profile")]
    let _profiler = dhat::Profiler::new_heap();

    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <path-to-analyze>", args[0]);
        eprintln!("\nRun with: cargo run --example heap_profile --features heap-profile -- <path>");
        std::process::exit(1);
    }

    let path = PathBuf::from(&args[1]);

    println!("Analyzing: {}", path.display());

    // Create analyzer with default settings
    let mut analyzer = CytoScnPy::default()
        .with_secrets(false)
        .with_danger(false)
        .with_quality(false);

    // Run analysis
    let summary = analyzer.analyze(&path);

    println!("✓ Analysis complete!");
    println!("  Files: {}", summary.analysis_summary.total_files);
    println!("  Lines: {}", summary.analysis_summary.total_lines_analyzed);
    println!(
        "  Definitions: {}",
        summary.analysis_summary.total_definitions
    );
    let unused_count = summary.unused_functions.len()
        + summary.unused_methods.len()
        + summary.unused_imports.len()
        + summary.unused_classes.len()
        + summary.unused_variables.len()
        + summary.unused_parameters.len();
    println!("  Unused: {unused_count}");

    #[cfg(feature = "heap-profile")]
    {
        println!("\n📊 Heap profile written to: dhat-heap.json");
        println!("   View at: https://nnethercote.github.io/dh_view/dh_view.html");
    }

    #[cfg(not(feature = "heap-profile"))]
    {
        println!("\n⚠️  Heap profiling not enabled!");
        println!("   Run with: cargo run --example heap_profile --features heap-profile -- <path>");
    }
}
