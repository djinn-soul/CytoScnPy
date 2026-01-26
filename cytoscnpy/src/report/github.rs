use crate::analyzer::AnalysisResult;
use crate::rules::Finding;
use std::io::Write;

/// Generates `GitHub Actions` workflow commands.
///
/// See: <https://docs.github.com/en/actions/using-workflows/workflow-commands-for-github-actions>
///
/// # Errors
///
/// Returns an error if writing to the `writer` fails.
pub fn print_github(writer: &mut impl Write, result: &AnalysisResult) -> std::io::Result<()> {
    print_github_with_root(writer, result, None)
}

/// Generates `GitHub Actions` workflow commands with an optional root path.
///
/// # Errors
///
/// Returns an error if writing to the `writer` fails.
pub fn print_github_with_root(
    writer: &mut impl Write,
    result: &AnalysisResult,
    root: Option<&std::path::Path>,
) -> std::io::Result<()> {
    // Security Findings
    for finding in &result.danger {
        write_annotation(writer, "error", finding, root)?;
    }
    // Secrets
    for secret in &result.secrets {
        // Secrets are always errors
        let path = normalize_path(&secret.file, root);
        writeln!(
            writer,
            "::error file={},line={},title={}::{}",
            path, secret.line, secret.rule_id, secret.message
        )?;
    }
    // Quality Findings
    for finding in &result.quality {
        write_annotation(writer, "warning", finding, root)?;
    }
    // Taint Findings
    for finding in &result.taint_findings {
        // Handle TaintFinding manually since it differs from Finding
        let path = normalize_path(&finding.file, root);
        writeln!(
            writer,
            "::warning file={},line={},col={},title={}::{} (Source: {})",
            path,
            finding.sink_line,
            finding.sink_col,
            finding.rule_id,
            finding.vuln_type,
            finding.source
        )?;
    }

    // Unused Code - usually warnings
    for func in &result.unused_functions {
        write_unused(
            writer,
            "warning",
            "UnusedFunction",
            &func.file,
            func.line,
            func.col,
            &func.name,
            root,
        )?;
    }
    for cls in &result.unused_classes {
        write_unused(
            writer,
            "warning",
            "UnusedClass",
            &cls.file,
            cls.line,
            cls.col,
            &cls.name,
            root,
        )?;
    }
    for imp in &result.unused_imports {
        write_unused(
            writer,
            "warning",
            "UnusedImport",
            &imp.file,
            imp.line,
            imp.col,
            &imp.name,
            root,
        )?;
    }
    for var in &result.unused_variables {
        write_unused(
            writer,
            "warning",
            "UnusedVariable",
            &var.file,
            var.line,
            var.col,
            &var.name,
            root,
        )?;
    }
    for method in &result.unused_methods {
        write_unused(
            writer,
            "warning",
            "UnusedMethod",
            &method.file,
            method.line,
            method.col,
            &method.name,
            root,
        )?;
    }
    for param in &result.unused_parameters {
        write_unused(
            writer,
            "warning",
            "UnusedParameter",
            &param.file,
            param.line,
            param.col,
            &param.name,
            root,
        )?;
    }

    // Parse Errors
    for error in &result.parse_errors {
        let path = normalize_path(&error.file, root);

        // Try to extract line number from message: "... at line 5"
        let line_meta = if let Some(idx) = error.error.rfind(" at line ") {
            if let Ok(line_num) = error.error[idx + 9..].parse::<usize>() {
                format!(",line={line_num}")
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        writeln!(
            writer,
            "::error file={}{},title=ParseError::{}",
            path, line_meta, error.error
        )?;
    }

    Ok(())
}

fn normalize_path(path: &std::path::Path, root: Option<&std::path::Path>) -> String {
    let normalized = if let Some(r) = root {
        // Handle common root cases robustly
        if r.as_os_str() == "." || r.as_os_str().is_empty() {
            path
        } else {
            path.strip_prefix(r).unwrap_or(path)
        }
    } else {
        path
    };
    let s = normalized.to_string_lossy().replace('\\', "/");
    s.strip_prefix("./").unwrap_or(&s).to_owned()
}

fn write_annotation(
    writer: &mut impl Write,
    level: &str,
    finding: &Finding,
    root: Option<&std::path::Path>,
) -> std::io::Result<()> {
    // Map severity to level if needed, but 'level' arg overrides for now based on category
    // GitHub supports: debug, notice, warning, error
    let gh_level = match finding.severity.to_uppercase().as_str() {
        "CRITICAL" | "HIGH" => "error",
        _ => level,
    };

    let path = normalize_path(&finding.file, root);

    writeln!(
        writer,
        "::{} file={},line={},col={},title={}::{} ({}:{})",
        gh_level, path, finding.line, finding.col, finding.rule_id, finding.message, path, finding.line
    )?;
    Ok(())
}

fn write_unused(
    writer: &mut impl Write,
    level: &str,
    title: &str,
    file: &std::path::Path,
    line: usize,
    col: usize,
    name: &str,
    root: Option<&std::path::Path>,
) -> std::io::Result<()> {
    let path = normalize_path(file, root);
    writeln!(
        writer,
        "::{level} file={path},line={line},col={col},title={title}::Unused identifier '{name}' in {path}:{line}"
    )?;
    Ok(())
}
