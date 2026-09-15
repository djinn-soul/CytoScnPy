//! Human-readable unified `DeSlopify` report composition.

use anyhow::{Context, Result};
use std::io::Write;

use super::GateFailure;

pub(super) fn render_human_report(
    architecture: &crate::architecture::ArchitectureGraphResult,
    context: &crate::context::ContextAnalysisResult,
    health: &[crate::doctor::DoctorResult],
    searchability: &crate::searchability::SearchabilityResult,
    failures: &[GateFailure],
) -> Result<String> {
    let mut output = Vec::new();
    writeln!(output, "# Architecture")?;
    crate::architecture::print_terminal_report(architecture, false, &mut output)?;
    writeln!(output, "\n# Git Context and Hotspots")?;
    crate::context::print_terminal_report(context, false, &mut output)?;

    for result in health {
        writeln!(
            output,
            "\n# Repository Health: {}",
            result.root_path.display()
        )?;
        crate::doctor::print_terminal_report(result, &mut output)?;
    }

    writeln!(output, "\n# Codebase Searchability")?;
    crate::searchability::print_terminal_report(searchability, None, &mut output)?;

    writeln!(output, "\n# CI Gates")?;
    if failures.is_empty() {
        writeln!(output, "PASSED")?;
    } else {
        writeln!(output, "FAILED")?;
        for failure in failures {
            writeln!(
                output,
                "- {}: {} (limit {})",
                failure.check, failure.actual, failure.limit
            )?;
        }
    }

    String::from_utf8(output).context("comprehensive report contained invalid UTF-8")
}
