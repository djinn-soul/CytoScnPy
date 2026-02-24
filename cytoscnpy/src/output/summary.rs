use crate::analyzer::{AnalysisResult, AnalysisSummary};
use colored::Colorize;
use std::io::Write;

/// Print the main header with box-drawing characters.
///
/// # Errors
///
/// Returns an error if writing to the output fails.
pub fn print_header(writer: &mut impl Write) -> std::io::Result<()> {
    writeln!(writer)?;
    writeln!(
        writer,
        "{}",
        "╔════════════════════════════════════════╗".cyan()
    )?;
    writeln!(
        writer,
        "{}",
        "║  Python Static Analysis Results        ║".cyan().bold()
    )?;
    writeln!(
        writer,
        "{}",
        "╚════════════════════════════════════════╝".cyan()
    )?;
    writeln!(writer)?;
    Ok(())
}

/// Print summary with colored "pills".
///
/// # Errors
///
/// Returns an error if writing to the output fails.
pub fn print_summary_pills(
    writer: &mut impl Write,
    result: &AnalysisResult,
) -> std::io::Result<()> {
    fn pill(label: &str, count: usize) -> String {
        if count == 0 {
            format!("{}: {}", label, count.to_string().green())
        } else {
            format!("{}: {}", label, count.to_string().red().bold())
        }
    }

    writeln!(
        writer,
        "{}  {}  {}  {}  {}  {}",
        pill("Unreachable", result.unused_functions.len()),
        pill("Methods", result.unused_methods.len()),
        pill("Imports", result.unused_imports.len()),
        pill("Params", result.unused_parameters.len()),
        pill("Vars", result.unused_variables.len()),
        pill("Classes", result.unused_classes.len()),
    )?;

    writeln!(
        writer,
        "{}  {}  {}  {}  {}",
        pill("Security", result.danger.len()),
        pill("Secrets", result.secrets.len()),
        pill("Quality", result.quality.len()),
        pill("Taint", result.taint_findings.len()),
        pill("Parse Errors", result.parse_errors.len()),
    )?;

    writeln!(writer)?;
    Ok(())
}

/// Print analysis statistics (files and lines processed).
///
/// # Errors
///
/// Returns an error if writing to the output fails.
pub fn print_analysis_stats(
    writer: &mut impl Write,
    summary: &AnalysisSummary,
) -> std::io::Result<()> {
    writeln!(
        writer,
        "{}",
        format!(
            "Analyzed {} files ({} lines)",
            summary.total_files.to_string().bold(),
            summary.total_lines_analyzed.to_string().bold()
        )
        .dimmed()
    )?;

    if summary.average_complexity > 0.0 || summary.average_mi > 0.0 {
        let complexity_color = if summary.average_complexity > 10.0 {
            colored::Color::Red
        } else {
            colored::Color::Green
        };
        let mi_color = if summary.average_mi < 40.0 {
            colored::Color::Red
        } else {
            colored::Color::Green
        };

        writeln!(
            writer,
            "Average Complexity: {} | Average MI: {}",
            format!("{:.2}", summary.average_complexity)
                .color(complexity_color)
                .bold(),
            format!("{:.2}", summary.average_mi).color(mi_color).bold()
        )?;
    }
    writeln!(writer)?;
    Ok(())
}
