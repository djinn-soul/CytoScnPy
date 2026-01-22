use crate::analyzer::AnalysisResult;
use std::io::Write;

/// Generates a Markdown summary of findings.
pub fn print_markdown(writer: &mut impl Write, result: &AnalysisResult) -> std::io::Result<()> {
    print_markdown_with_root(writer, result, None)
}

/// Generates a Markdown summary of findings with an optional root path.
pub fn print_markdown_with_root(
    writer: &mut impl Write,
    result: &AnalysisResult,
    _root: Option<&std::path::Path>,
) -> std::io::Result<()> {
    writeln!(writer, "# CytoScnPy Analysis Report\n")?;

    // Summary Table
    writeln!(writer, "## Summary\n")?;
    writeln!(writer, "| Category | Count |")?;
    writeln!(writer, "| --- | ---: |")?;
    writeln!(writer, "| Security Issues | {} |", result.danger.len())?;
    writeln!(writer, "| Secrets Found | {} |", result.secrets.len())?;
    writeln!(writer, "| Quality Issues | {} |", result.quality.len())?;
    writeln!(writer, "| Taint Issues | {} |", result.taint_findings.len())?;
    writeln!(
        writer,
        "| Unused Functions | {} |",
        result.unused_functions.len()
    )?;
    writeln!(
        writer,
        "| Unused Methods | {} |",
        result.unused_methods.len()
    )?;
    writeln!(
        writer,
        "| Unused Classes | {} |",
        result.unused_classes.len()
    )?;
    writeln!(
        writer,
        "| Unused Imports | {} |",
        result.unused_imports.len()
    )?;
    writeln!(
        writer,
        "| Unused Parameters | {} |",
        result.unused_parameters.len()
    )?;
    writeln!(
        writer,
        "| Unused Variables | {} |",
        result.unused_variables.len()
    )?;
    writeln!(writer, "| Parse Errors | {} |", result.parse_errors.len())?;
    writeln!(writer)?;

    // Security Details
    if !result.danger.is_empty() {
        writeln!(writer, "## Security Issues\n")?;
        writeln!(writer, "| Rule | File | Line | Message | Severity |")?;
        writeln!(writer, "| --- | --- | ---: | --- | --- |")?;
        for f in &result.danger {
            writeln!(
                writer,
                "| {} | {} | {} | {} | {} |",
                f.rule_id,
                f.file.display(),
                f.line,
                f.message,
                f.severity
            )?;
        }
        writeln!(writer)?;
    }

    // Secrets Details
    if !result.secrets.is_empty() {
        writeln!(writer, "## Secrets Detected\n")?;
        writeln!(writer, "| Rule | File | Line | Message |")?;
        writeln!(writer, "| --- | --- | ---: | --- |")?;
        for s in &result.secrets {
            writeln!(
                writer,
                "| {} | {} | {} | {} |",
                s.rule_id,
                s.file.display(),
                s.line,
                s.message
            )?;
        }
        writeln!(writer)?;
    }

    // Quality Details
    if !result.quality.is_empty() {
        writeln!(writer, "## Quality Issues\n")?;
        writeln!(writer, "| Rule | File | Line | Message | Severity |")?;
        writeln!(writer, "| --- | --- | ---: | --- | --- |")?;
        for f in &result.quality {
            writeln!(
                writer,
                "| {} | {} | {} | {} | {} |",
                f.rule_id,
                f.file.display(),
                f.line,
                f.message,
                f.severity
            )?;
        }
        writeln!(writer)?;
    }

    // Taint Details
    if !result.taint_findings.is_empty() {
        writeln!(writer, "## Taint Issues\n")?;
        writeln!(writer, "| Rule | File | Line | Message | Severity |")?;
        writeln!(writer, "| --- | --- | ---: | --- | --- |")?;
        for f in &result.taint_findings {
            writeln!(
                writer,
                "| {} | {} | {} | {} (Source: {}) | {} |",
                f.rule_id,
                f.file.display(),
                f.sink_line,
                f.vuln_type,
                f.source,
                f.severity.to_string()
            )?;
        }
        writeln!(writer)?;
    }

    // Unused Code Details
    if !result.unused_functions.is_empty()
        || !result.unused_methods.is_empty()
        || !result.unused_classes.is_empty()
        || !result.unused_imports.is_empty()
        || !result.unused_variables.is_empty()
        || !result.unused_parameters.is_empty()
    {
        writeln!(writer, "## Unused Code\n")?;
        writeln!(writer, "| Type | Name | File | Line |")?;
        writeln!(writer, "| --- | --- | --- | ---: |")?;

        for f in &result.unused_functions {
            writeln!(
                writer,
                "| Function | {} | {} | {} |",
                f.name,
                f.file.display(),
                f.line
            )?;
        }
        for m in &result.unused_methods {
            writeln!(
                writer,
                "| Method | {} | {} | {} |",
                m.name,
                m.file.display(),
                m.line
            )?;
        }
        for c in &result.unused_classes {
            writeln!(
                writer,
                "| Class | {} | {} | {} |",
                c.name,
                c.file.display(),
                c.line
            )?;
        }
        for i in &result.unused_imports {
            writeln!(
                writer,
                "| Import | {} | {} | {} |",
                i.name,
                i.file.display(),
                i.line
            )?;
        }
        for v in &result.unused_variables {
            writeln!(
                writer,
                "| Variable | {} | {} | {} |",
                v.simple_name,
                v.file.display(),
                v.line
            )?;
        }
        for p in &result.unused_parameters {
            writeln!(
                writer,
                "| Parameter | {} | {} | {} |",
                p.simple_name,
                p.file.display(),
                p.line
            )?;
        }
        writeln!(writer)?;
    }

    // Parse Errors
    if !result.parse_errors.is_empty() {
        writeln!(writer, "## Parse Errors\n")?;
        writeln!(writer, "| File | Error |")?;
        writeln!(writer, "| --- | --- |")?;
        for e in &result.parse_errors {
            writeln!(writer, "| {} | {} |", e.file.display(), e.error)?;
        }
        writeln!(writer)?;
    }

    Ok(())
}
