//! Human-readable unified `DeSlopify` report composition.

use anyhow::{Context, Result};
use std::io::Write;

use super::GateFailure;

pub(super) fn render_human_report(
    architecture: &crate::architecture::ArchitectureGraphResult,
    context: &crate::context::ContextAnalysisResult,
    health: &[crate::doctor::DoctorResult],
    searchability: &crate::searchability::SearchabilityResult,
    naming: &crate::naming::NamingDistributionResult,
    todos: &crate::todos::TodosResult,
    globals: &crate::globals::GlobalsResult,
    exceptions: &crate::exceptions::ExceptionsResult,
    wildcards: &crate::wildcards::WildcardsResult,
    side_effects: &crate::side_effects::SideEffectsResult,
    singletons: &crate::singletons::SingletonsResult,
    anti_patterns: &crate::anti_patterns::AntiPatternsResult,
    duplicates: &crate::duplicates::DuplicatesResult,
    unreferenced: &crate::unreferenced::UnreferencedResult,
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

    writeln!(output, "\n# Naming Style Consistency")?;
    crate::naming::print_terminal_report(naming, None, &mut output)?;

    writeln!(output, "\n# TODO / Annotation Scan")?;
    crate::todos::print_terminal_report(todos, None, &mut output)?;

    writeln!(output, "\n# Mutable Global State")?;
    crate::globals::print_terminal_report(globals, None, &mut output)?;

    writeln!(output, "\n# Exception Handler Anti-Patterns")?;
    crate::exceptions::print_terminal_report(exceptions, None, &mut output)?;

    writeln!(output, "\n# Wildcard Imports")?;
    crate::wildcards::print_terminal_report(wildcards, None, &mut output)?;

    writeln!(output, "\n# Module-Level Side Effects")?;
    crate::side_effects::print_terminal_report(side_effects, None, &mut output)?;

    writeln!(output, "\n# Singleton Patterns")?;
    crate::singletons::print_terminal_report(singletons, None, &mut output)?;

    writeln!(output, "\n# Anti-Patterns")?;
    crate::anti_patterns::print_terminal_report(anti_patterns, None, &mut output)?;

    writeln!(output, "\n# Duplicate Code Clusters")?;
    crate::duplicates::print_terminal_report(duplicates, None, &mut output)?;

    writeln!(output, "\n# Unreferenced Large Functions in Isolated Files")?;
    crate::unreferenced::print_terminal_report(unreferenced, None, &mut output)?;

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

pub(super) fn write_report<W: Write, T: serde::Serialize>(
    report: &T,
    human_output: Option<&str>,
    args: &crate::cli::DeslopArgs,
    analysis_root: &std::path::Path,
    json: bool,
    writer: &mut W,
) -> Result<()> {
    let payload = match human_output {
        Some(output) => output.to_owned(),
        None => serde_json::to_string_pretty(report)?,
    };
    if let Some(output) = &args.output {
        let path =
            crate::utils::validate_output_path(std::path::Path::new(output), Some(analysis_root))?;
        std::fs::write(&path, payload)?;
        if !json {
            writeln!(writer, "Report written to: {}", path.display())?;
        }
    } else {
        writeln!(writer, "{payload}")?;
    }
    Ok(())
}
