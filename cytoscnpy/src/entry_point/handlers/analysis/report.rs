use anyhow::Result;
use colored::Colorize;
use serde_json::{Map, Value};

use super::context::AnalysisContext;
use super::run::AnalysisRun;

pub(crate) fn report_results<W: std::io::Write>(
    cli_var: &crate::cli::Cli,
    analysis_root: &std::path::Path,
    context: &AnalysisContext,
    run: &AnalysisRun,
    writer: &mut W,
) -> Result<()> {
    let result = &run.result;

    if cli_var.output.verbose && !context.is_structured {
        print_verbose_stats(result, run);
    }

    output_results(cli_var, analysis_root, result, writer)?;

    if !context.is_structured {
        print_summary(cli_var, result, run, writer)?;
    }

    #[cfg(feature = "html_report")]
    maybe_generate_html_report(cli_var, analysis_root, result, writer)?;

    Ok(())
}

fn print_verbose_stats(result: &crate::analyzer::AnalysisResult, run: &AnalysisRun) {
    let elapsed = run.start_time.elapsed();
    eprintln!(
        "[VERBOSE] Analysis completed in {:.2}s",
        elapsed.as_secs_f64()
    );
    eprintln!("   Files analyzed: {}", result.analysis_summary.total_files);
    eprintln!(
        "   Lines analyzed: {}",
        result.analysis_summary.total_lines_analyzed
    );
    eprintln!("[VERBOSE] Findings breakdown:");
    eprintln!(
        "   Unreachable functions: {}",
        result.unused_functions.len()
    );
    eprintln!("   Unreachable methods: {}", result.unused_methods.len());
    eprintln!("   Unused classes: {}", result.unused_classes.len());
    eprintln!("   Unused imports: {}", result.unused_imports.len());
    eprintln!("   Unused variables: {}", result.unused_variables.len());
    eprintln!("   Unused parameters: {}", result.unused_parameters.len());
    eprintln!("   Parse errors: {}", result.parse_errors.len());

    let mut file_counts = collect_issue_counts(result);
    if !file_counts.is_empty() {
        file_counts.sort_by(|a, b| b.1.cmp(&a.1));
        eprintln!("[VERBOSE] Files with most issues:");
        for (file, count) in file_counts.iter().take(5) {
            eprintln!("   {count:3} issues: {file}");
        }
    }
    eprintln!();
}

fn collect_issue_counts(result: &crate::analyzer::AnalysisResult) -> Vec<(String, usize)> {
    let mut file_counts = std::collections::HashMap::new();
    for item in &result.unused_functions {
        *file_counts
            .entry(crate::utils::normalize_display_path(&item.file))
            .or_insert(0) += 1;
    }
    for item in &result.unused_methods {
        *file_counts
            .entry(crate::utils::normalize_display_path(&item.file))
            .or_insert(0) += 1;
    }
    for item in &result.unused_classes {
        *file_counts
            .entry(crate::utils::normalize_display_path(&item.file))
            .or_insert(0) += 1;
    }
    for item in &result.unused_imports {
        *file_counts
            .entry(crate::utils::normalize_display_path(&item.file))
            .or_insert(0) += 1;
    }
    for item in &result.unused_variables {
        *file_counts
            .entry(crate::utils::normalize_display_path(&item.file))
            .or_insert(0) += 1;
    }
    for item in &result.unused_parameters {
        *file_counts
            .entry(crate::utils::normalize_display_path(&item.file))
            .or_insert(0) += 1;
    }

    file_counts.into_iter().collect()
}

fn output_results<W: std::io::Write>(
    cli_var: &crate::cli::Cli,
    analysis_root: &std::path::Path,
    result: &crate::analyzer::AnalysisResult,
    writer: &mut W,
) -> Result<()> {
    if cli_var.output.json || cli_var.output.format == crate::cli::OutputFormat::Json {
        writeln!(writer, "{}", machine_json_payload(result)?)?;
        return Ok(());
    }

    if cli_var.clones && cli_var.output.verbose {
        eprintln!("[VERBOSE] Clone detection enabled");
        eprintln!(
            "   Similarity threshold: {:.0}%",
            cli_var.clone_similarity * 100.0
        );
        eprintln!();
    }

    match cli_var.output.format {
        crate::cli::OutputFormat::Text => {
            #[cfg(feature = "html_report")]
            let show_cli = !cli_var.output.html;
            #[cfg(not(feature = "html_report"))]
            let show_cli = true;

            if show_cli {
                if cli_var.output.quiet {
                    crate::output::print_report_quiet(writer, result)?;
                } else {
                    crate::output::print_report(writer, result)?;
                }
            }
        }
        crate::cli::OutputFormat::Grouped => {
            crate::output::print_report_grouped(writer, result)?;
        }
        crate::cli::OutputFormat::Junit => {
            crate::report::junit::print_junit_with_root(writer, result, Some(analysis_root))?;
        }
        crate::cli::OutputFormat::Github => {
            crate::report::github::print_github_with_root(writer, result, Some(analysis_root))?;
        }
        crate::cli::OutputFormat::Gitlab => {
            crate::report::gitlab::print_gitlab_with_root(writer, result, Some(analysis_root))?;
        }
        crate::cli::OutputFormat::Markdown => {
            crate::report::markdown::print_markdown_with_root(writer, result, Some(analysis_root))?;
        }
        crate::cli::OutputFormat::Sarif => {
            crate::report::sarif::print_sarif_with_root(writer, result, Some(analysis_root))?;
        }
        crate::cli::OutputFormat::Json => unreachable!(),
    }

    Ok(())
}

fn machine_json_payload(result: &crate::analyzer::AnalysisResult) -> Result<String> {
    let mut root = match serde_json::to_value(result)? {
        Value::Object(map) => map,
        _ => Map::new(),
    };
    root.insert("schema_version".to_owned(), Value::String("2".to_owned()));
    root.insert(
        "stable_findings".to_owned(),
        Value::Array(stable_findings(result)),
    );
    Ok(serde_json::to_string_pretty(&Value::Object(root))?)
}

fn stable_findings(result: &crate::analyzer::AnalysisResult) -> Vec<Value> {
    let mut items = Vec::new();

    for def in &result.unused_functions {
        items.push(stable_dead_code_item("unused_function", def));
    }
    for def in &result.unused_methods {
        items.push(stable_dead_code_item("unused_method", def));
    }
    for def in &result.unused_classes {
        items.push(stable_dead_code_item("unused_class", def));
    }
    for def in &result.unused_imports {
        items.push(stable_dead_code_item("unused_import", def));
    }
    for def in &result.unused_variables {
        items.push(stable_dead_code_item("unused_variable", def));
    }
    for def in &result.unused_parameters {
        items.push(stable_dead_code_item("unused_parameter", def));
    }
    for finding in &result.danger {
        let file = crate::utils::normalize_display_path(&finding.file);
        let stable_id = format!(
            "danger:{}:{}:{}:{}",
            finding.rule_id, file, finding.line, finding.col
        );
        items.push(serde_json::json!({
            "stable_id": stable_id,
            "kind": "danger",
            "rule_id": finding.rule_id,
            "file": file,
            "line": finding.line,
            "col": finding.col,
            "severity": finding.severity,
        }));
    }
    for finding in &result.secrets {
        items.push(stable_secret_item(finding));
    }
    for finding in &result.quality {
        let file = crate::utils::normalize_display_path(&finding.file);
        let stable_id = format!(
            "quality:{}:{}:{}:{}",
            finding.rule_id, file, finding.line, finding.col
        );
        items.push(serde_json::json!({
            "stable_id": stable_id,
            "kind": "quality",
            "rule_id": finding.rule_id,
            "file": file,
            "line": finding.line,
            "col": finding.col,
            "severity": finding.severity,
        }));
    }
    for finding in &result.taint_findings {
        let file = crate::utils::normalize_display_path(&finding.file);
        let stable_id = format!(
            "taint:{}:{}:{}:{}",
            finding.rule_id, file, finding.source_line, finding.sink_line
        );
        items.push(serde_json::json!({
            "stable_id": stable_id,
            "kind": "taint",
            "rule_id": finding.rule_id,
            "file": file,
            "source_line": finding.source_line,
            "sink_line": finding.sink_line,
            "severity": finding.severity.to_string(),
            "exploitability_score": finding.exploitability_score,
        }));
    }
    for error in &result.parse_errors {
        let file = crate::utils::normalize_display_path(&error.file);
        let stable_id = format!("parse_error:{file}:{}", error.error);
        items.push(serde_json::json!({
            "stable_id": stable_id,
            "kind": "parse_error",
            "file": file,
            "error": error.error,
        }));
    }

    items.sort_by(|a, b| {
        a.get("stable_id")
            .and_then(Value::as_str)
            .cmp(&b.get("stable_id").and_then(Value::as_str))
    });
    items
}

fn stable_dead_code_item(kind: &str, def: &crate::visitor::Definition) -> Value {
    let file = crate::utils::normalize_display_path(&def.file);
    let stable_id = format!("{kind}:{}:{}:{}", def.name, file, def.start_byte);
    serde_json::json!({
        "stable_id": stable_id,
        "kind": kind,
        "name": def.name,
        "file": file,
        "line": def.line,
        "start_byte": def.start_byte,
        "end_byte": def.end_byte,
        "confidence": def.confidence,
    })
}

fn stable_secret_item(finding: &crate::rules::secrets::SecretFinding) -> Value {
    let file = crate::utils::normalize_display_path(&finding.file);
    let matched = finding.matched_value.as_deref().unwrap_or("-");
    let entropy = finding
        .entropy
        .map(|value| format!("{value:.4}"))
        .unwrap_or_else(|| "-".to_owned());
    let stable_id = format!(
        "secret:{}:{}:{}:{}:{}:{}:{}",
        finding.rule_id, file, finding.line, finding.confidence, finding.message, matched, entropy
    );
    serde_json::json!({
        "stable_id": stable_id,
        "kind": "secret",
        "rule_id": finding.rule_id,
        "file": file,
        "line": finding.line,
        "severity": finding.severity,
        "confidence": finding.confidence,
    })
}

#[cfg(test)]
mod tests {
    use super::stable_findings;

    #[test]
    fn secret_stable_ids_include_disambiguators() {
        let mut result = crate::analyzer::AnalysisResult::default();
        let file = std::path::PathBuf::from("test.py");
        result.secrets.push(crate::rules::secrets::SecretFinding {
            message: "Hardcoded password".to_owned(),
            rule_id: "CSP-S001".to_owned(),
            category: "Secrets".to_owned(),
            file: file.clone(),
            line: 20,
            severity: "CRITICAL".to_owned(),
            matched_value: Some("abcd...wxyz".to_owned()),
            entropy: None,
            confidence: 100,
        });
        result.secrets.push(crate::rules::secrets::SecretFinding {
            message: "Hardcoded password variant".to_owned(),
            rule_id: "CSP-S001".to_owned(),
            category: "Secrets".to_owned(),
            file,
            line: 20,
            severity: "CRITICAL".to_owned(),
            matched_value: Some("1234...7890".to_owned()),
            entropy: None,
            confidence: 95,
        });

        let stable_ids: Vec<String> = stable_findings(&result)
            .into_iter()
            .filter(|item| item.get("kind").and_then(serde_json::Value::as_str) == Some("secret"))
            .filter_map(|item| {
                item.get("stable_id")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_owned)
            })
            .collect();

        assert_eq!(stable_ids.len(), 2);
        assert_ne!(stable_ids[0], stable_ids[1]);
    }
}

fn print_summary<W: std::io::Write>(
    cli_var: &crate::cli::Cli,
    result: &crate::analyzer::AnalysisResult,
    run: &AnalysisRun,
    writer: &mut W,
) -> Result<()> {
    let total = result.unused_functions.len()
        + result.unused_methods.len()
        + result.unused_imports.len()
        + result.unused_parameters.len()
        + result.unused_classes.len()
        + result.unused_variables.len();
    let security = result.danger.len() + result.secrets.len() + result.quality.len();

    if run.clone_pairs_found > 0 {
        writeln!(
            writer,
            "\n[SUMMARY] {total} unused code issues, {security} security/quality issues, {} clone pairs",
            run.clone_pairs_found
        )?;
    } else if !cli_var.output.quiet {
        writeln!(
            writer,
            "\n[SUMMARY] {total} unused code issues, {security} security/quality issues"
        )?;
    }

    if !cli_var.output.quiet {
        crate::output::print_summary_pills(writer, result)?;
        crate::output::print_analysis_stats(writer, &result.analysis_summary)?;
    }

    writeln!(
        writer,
        "{} in {:.2}s",
        "Analysis completed".green().bold(),
        run.start_time.elapsed().as_secs_f64()
    )?;

    Ok(())
}

#[cfg(feature = "html_report")]
fn maybe_generate_html_report<W: std::io::Write>(
    cli_var: &crate::cli::Cli,
    analysis_root: &std::path::Path,
    result: &crate::analyzer::AnalysisResult,
    writer: &mut W,
) -> Result<()> {
    if !cli_var.output.html {
        return Ok(());
    }

    writeln!(writer, "Generating HTML report...")?;
    let report_dir = std::path::Path::new(".cytoscnpy/report");
    if let Err(e) = crate::report::generator::generate_report(result, analysis_root, report_dir) {
        eprintln!("Failed to generate HTML report: {e}");
    } else {
        writeln!(writer, "HTML report generated at: {}", report_dir.display())?;
        if let Err(e) = open::that(report_dir.join("index.html")) {
            eprintln!("Failed to open report in browser: {e}");
        }
    }

    Ok(())
}
