use colored::Colorize;
use comfy_table::{Cell, Color, Table};
use std::io::Write;

use super::types::{ContextAnalysisResult, HotspotRiskLevel};

/// Formats and writes the terminal report.
pub fn print_terminal_report<W: Write>(
    result: &ContextAnalysisResult,
    hotspots_only: bool,
    mut writer: W,
) -> std::io::Result<()> {
    if !hotspots_only {
        print_summary_table(result, &mut writer)?;
        writeln!(writer)?;
        print_hot_files_table(result, &mut writer)?;
        writeln!(writer)?;
    }

    print_hotspots_table(result, &mut writer)?;

    if !hotspots_only {
        writeln!(writer)?;
        print_token_budget_table(result, &mut writer)?;
    }

    Ok(())
}

fn print_summary_table<W: Write>(
    result: &ContextAnalysisResult,
    writer: &mut W,
) -> std::io::Result<()> {
    writeln!(writer, "{}", "=== Git Activity & Code Surface ===".bold())?;

    let mut table = Table::new();
    table.set_header(vec!["Metric", "Value"]);

    let git_status = if result.git_activity.is_git_repo {
        "Active Git Repository".green().to_string()
    } else {
        "Not a Git Repository (or Git disabled)"
            .yellow()
            .to_string()
    };
    table.add_row(vec!["Repository Status", &git_status]);

    if result.git_activity.is_git_repo {
        table.add_row(vec![
            "Lookback Window".to_owned(),
            result.git_activity.window_label.clone(),
        ]);
        table.add_row(vec![
            "Total Commits in Window".to_owned(),
            result.git_activity.total_commits.to_string(),
        ]);
    }

    let active_ratio = if result.total_files > 0 {
        (result.git_activity.active_files as f64 / result.total_files as f64) * 100.0
    } else {
        0.0
    };

    table.add_row(vec![
        "Active Surface (Recent Churn)".to_owned(),
        format!(
            "{} files ({:.1}%) - {} lines",
            result.git_activity.active_files, active_ratio, result.git_activity.active_lines
        ),
    ]);
    table.add_row(vec![
        "Frozen Surface (Untouched)".to_owned(),
        format!(
            "{} files - {} lines",
            result.git_activity.frozen_files, result.git_activity.frozen_lines
        ),
    ]);
    table.add_row(vec![
        "Total Scanned Files".to_owned(),
        format!(
            "{} files ({} lines)",
            result.total_files, result.total_lines
        ),
    ]);

    writeln!(writer, "{table}")
}

fn print_hot_files_table<W: Write>(
    result: &ContextAnalysisResult,
    writer: &mut W,
) -> std::io::Result<()> {
    writeln!(writer, "{}", "=== Top Hot Files (Most Churned) ===".bold())?;

    if result.git_activity.hot_files.is_empty() {
        writeln!(
            writer,
            "{}",
            "  No recent file modifications found in Git history.".dimmed()
        )?;
        return Ok(());
    }

    let mut table = Table::new();
    table.set_header(vec!["#", "File", "Commits", "Lines"]);

    for (idx, hot) in result.git_activity.hot_files.iter().enumerate() {
        table.add_row(vec![
            (idx + 1).to_string(),
            hot.display_path.clone(),
            hot.commit_count.to_string(),
            hot.lines.to_string(),
        ]);
    }

    writeln!(writer, "{table}")
}

fn print_hotspots_table<W: Write>(
    result: &ContextAnalysisResult,
    writer: &mut W,
) -> std::io::Result<()> {
    writeln!(writer, "{}", "=== Churn-Complexity Hotspots ===".bold())?;

    if result.hotspots.is_empty() {
        writeln!(
            writer,
            "{}",
            "  No high-churn complex hotspots detected.".green()
        )?;
        return Ok(());
    }

    let mut table = Table::new();
    table.set_header(vec!["Risk", "File", "Commits", "Complexity", "Risk Score"]);

    for spot in &result.hotspots {
        let (risk_cell, risk_color) = match spot.risk_level {
            HotspotRiskLevel::Critical => (Cell::new("CRITICAL"), Color::Red),
            HotspotRiskLevel::High => (Cell::new("HIGH"), Color::Yellow),
            HotspotRiskLevel::Medium => (Cell::new("MEDIUM"), Color::Cyan),
            HotspotRiskLevel::Low => (Cell::new("LOW"), Color::White),
        };

        table.add_row(vec![
            risk_cell.fg(risk_color),
            Cell::new(&spot.display_path),
            Cell::new(spot.commit_count.to_string()),
            Cell::new(spot.cyclomatic_complexity.to_string()),
            Cell::new(spot.risk_score.to_string()),
        ]);
    }

    writeln!(writer, "{table}")
}

fn print_token_budget_table<W: Write>(
    result: &ContextAnalysisResult,
    writer: &mut W,
) -> std::io::Result<()> {
    writeln!(writer, "{}", "=== LLM Context Navigation Budget ===".bold())?;

    let mut table = Table::new();
    table.set_header(vec!["Phase", "Estimated Tokens", "% of Budget"]);

    let b = &result.token_budget;
    let u = b.usable_context.max(1) as f64;

    table.add_row(vec![
        "File Discovery".to_owned(),
        format!("{} tokens", b.file_discovery_tokens),
        format!("{:.1}%", (b.file_discovery_tokens as f64 / u) * 100.0),
    ]);
    table.add_row(vec![
        "Code Reading".to_owned(),
        format!("{} tokens", b.code_reading_tokens),
        format!("{:.1}%", (b.code_reading_tokens as f64 / u) * 100.0),
    ]);
    table.add_row(vec![
        "Dependency Tracing".to_owned(),
        format!("{} tokens", b.dependency_tracing_tokens),
        format!("{:.1}%", (b.dependency_tracing_tokens as f64 / u) * 100.0),
    ]);
    table.add_row(vec![
        "Comprehension Overhead".to_owned(),
        format!("{} tokens", b.comprehension_tokens),
        format!("{:.1}%", (b.comprehension_tokens as f64 / u) * 100.0),
    ]);
    table.add_row(vec![
        "Total Navigation Cost".to_owned(),
        format!("{} tokens", b.total_navigation_tokens),
        format!("{:.1}%", b.navigation_pct),
    ]);
    table.add_row(vec![
        "Remaining Headroom".to_owned(),
        format!("{} tokens", b.remaining_tokens),
        format!("{:.1}%", 100.0 - b.navigation_pct),
    ]);
    table.add_row(vec![
        "Usable Context Window".to_owned(),
        format!("{} tokens", b.usable_context),
        format!("Verdict: {}", b.status_verdict),
    ]);

    writeln!(writer, "{table}")
}

/// Formats and writes the JSON report.
pub fn print_json_report<W: Write>(
    result: &ContextAnalysisResult,
    mut writer: W,
) -> std::io::Result<()> {
    let json = serde_json::to_string_pretty(result)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    writeln!(writer, "{json}")
}
