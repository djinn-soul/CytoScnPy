#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]

use crate::analyzer::AnalysisResult;
use crate::report::templates::{
    CategoryScore, CloneItem, ClonesTemplate, DashboardTemplate, FileViewTemplate, FilesTemplate,
    IssueItem, IssuesTemplate, OverallScore,
};
use anyhow::Result;
use askama::Template;
use std::fs;
use std::path::Path;

/// Generates a comprehensive HTML report based on the analysis results.
///
/// This function creates a report directory, copies static assets (CSS, JS),
/// and generates the main dashboard, issues view, and individual file views.
///
/// # Errors
///
/// Returns an error if directory creation, file I/O, or template rendering fails.
#[allow(clippy::too_many_lines)]
pub fn generate_report(result: &AnalysisResult, root: &Path, output_dir: &Path) -> Result<()> {
    let output_dir = crate::utils::validate_output_path(output_dir, Some(root))?;

    if !output_dir.exists() {
        fs::create_dir_all(&output_dir)?;
    }

    // 1. Calculate Score
    let score = calculate_score(result);

    // 2. Prepare Data
    let generated_at = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let version = env!("CARGO_PKG_VERSION").to_owned();

    let issue_items = flatten_issues(result);

    // 3. Build file metrics view
    let file_metrics_view: Vec<crate::report::templates::FileMetricsView> = result
        .file_metrics
        .iter()
        .map(|f| crate::report::templates::FileMetricsView {
            file: f.file.to_string_lossy().to_string(),
            sloc: f.sloc,
            complexity: f.complexity,
            raw_mi: f.mi,
            mi: format!("{:.1}", f.mi),
            total_issues: f.total_issues,
            link: format!(
                "files/{}.html",
                f.file.to_string_lossy().replace(['/', '\\', ':'], "_")
            ),
        })
        .collect();

    let score_color = if score.total_score >= 80 {
        "#4ade80".to_owned()
    } else {
        "#f87171".to_owned()
    };

    let average_mi_color = if result.analysis_summary.average_mi >= 65.0 {
        "#4ade80".to_owned()
    } else {
        "#f87171".to_owned()
    };

    let total_issues_color = if issue_items.is_empty() {
        "var(--text-main)".to_owned()
    } else {
        "var(--severity-high)".to_owned()
    };

    // 4. Generate Dashboard
    // Calculate Halstead Averages
    let file_count = result.analysis_summary.total_files.max(1) as f64;
    let h_metrics = &result.analysis_summary.halstead_metrics;

    let avg_vol = h_metrics.volume / file_count;
    let avg_diff = h_metrics.difficulty / file_count;
    let avg_effort = h_metrics.effort / file_count;
    let total_bugs = h_metrics.bugs;

    let (vol_level, vol_color, vol_icon) = if avg_vol < 1000.0 {
        ("Low", "var(--success)", "✓")
    } else {
        ("Very High", "var(--severity-high)", "⚠️")
    };
    let (diff_level, diff_color, diff_icon) = if avg_diff < 5.0 {
        ("Low", "var(--success)", "✓")
    } else if avg_diff < 10.0 {
        ("Moderate", "var(--warning)", "!")
    } else if avg_diff < 20.0 {
        ("High", "var(--severity-medium)", "⚡")
    } else {
        ("Very High", "var(--severity-high)", "⚠️")
    };
    let (eff_level, eff_color, eff_icon) = if avg_effort < 10000.0 {
        ("Low", "var(--success)", "✓")
    } else if avg_effort < 30000.0 {
        ("Moderate", "var(--warning)", "!")
    } else if avg_effort < 50000.0 {
        ("High", "var(--severity-medium)", "⚡")
    } else {
        ("Very High", "var(--severity-high)", "⚠️")
    };
    let (bugs_level, bugs_color, bugs_icon) = if total_bugs < 0.5 {
        ("Low", "var(--success)", "✓")
    } else if total_bugs < 2.0 {
        ("Moderate", "var(--warning)", "!")
    } else if total_bugs < 5.0 {
        ("High", "var(--severity-medium)", "⚡")
    } else {
        ("Very High", "var(--severity-high)", "⚠️")
    };

    let dashboard = DashboardTemplate {
        score,
        score_color,
        total_files: result.analysis_summary.total_files,
        total_lines: result.analysis_summary.total_lines_analyzed,
        total_issues: issue_items.len(),
        total_issues_color,
        unused_imports: result.unused_imports.len(),
        unused_functions: result.unused_functions.len(),
        unused_classes: result.unused_classes.len(),
        unused_variables: result.unused_variables.len(),
        unused_methods: result.unused_methods.len(),
        unused_parameters: result.unused_parameters.len(),
        average_mi_str: format!("{:.1}", result.analysis_summary.average_mi),
        average_mi_color: average_mi_color.clone(),
        summary: result.analysis_summary.clone(),
        halstead_view: crate::report::templates::FormattedHalsteadMetrics {
            volume: format!("{avg_vol:.2}"),
            volume_level: vol_level.to_owned(),
            volume_color: vol_color.to_owned(),
            volume_icon: vol_icon.to_owned(),
            difficulty: format!("{avg_diff:.2}"),
            difficulty_level: diff_level.to_owned(),
            difficulty_color: diff_color.to_owned(),
            difficulty_icon: diff_icon.to_owned(),
            effort: format!("{avg_effort:.2}"),
            effort_level: eff_level.to_owned(),
            effort_color: eff_color.to_owned(),
            effort_icon: eff_icon.to_owned(),
            bugs: format!("{total_bugs:.2}"),
            bugs_level: bugs_level.to_owned(),
            bugs_color: bugs_color.to_owned(),
            bugs_icon: bugs_icon.to_owned(),
            time: format!("{:.2}", h_metrics.time / file_count),
            calculated_length: format!("{:.2}", h_metrics.calculated_length / file_count),
        },
        generated_at: generated_at.clone(),
        version: version.clone(),
        root_path: ".".to_owned(),
    };
    fs::write(output_dir.join("index.html"), dashboard.render()?)?;

    // 5. Generate Issues Page
    let (unused, security, quality) = segregate_issues(&issue_items);
    let issues_page = IssuesTemplate {
        unused_code: unused,
        securityable: security,
        quality,
        generated_at: generated_at.clone(),
        version: version.clone(),
        root_path: ".".to_owned(),
    };
    fs::write(output_dir.join("issues.html"), issues_page.render()?)?;

    // 6. Generate Files Page
    let files_page = FilesTemplate {
        file_metrics: file_metrics_view,
        average_mi: format!("{:.1}", result.analysis_summary.average_mi),
        average_mi_color,
        version: version.clone(),
        generated_at: generated_at.clone(),
        root_path: ".".to_owned(),
    };
    fs::write(output_dir.join("files.html"), files_page.render()?)?;

    // 7. Generate Clones Page
    let clone_items: Vec<CloneItem> = result
        .clones
        .iter()
        .filter(|c| c.is_duplicate)
        .map(|c| {
            let safe_file = c.file.to_string_lossy().replace(['/', '\\', ':'], "_") + ".html";
            let safe_related = c
                .related_clone
                .file
                .to_string_lossy()
                .replace(['/', '\\', ':'], "_")
                + ".html";

            CloneItem {
                similarity: c.similarity,
                clone_type: c.clone_type.display_name().to_owned(),
                name: c.name.clone().unwrap_or_else(|| "<anonymous>".to_owned()),
                file: c.file.to_string_lossy().to_string(),
                line: c.line,
                link: format!("files/{}#L{}", safe_file, c.line),
                related_file: c.related_clone.file.to_string_lossy().to_string(),
                related_line: c.related_clone.line,
                related_link: format!("files/{}#L{}", safe_related, c.related_clone.line),
            }
        })
        .collect();

    let clones_page = ClonesTemplate {
        clones: clone_items,
        generated_at: generated_at.clone(),
        version: version.clone(),
        root_path: ".".to_owned(),
    };
    fs::write(output_dir.join("clones.html"), clones_page.render()?)?;

    // 7. Generate Assets (CSS/JS)
    write_assets(&output_dir)?;

    // 8. Generate File Views
    generate_file_views(result, &issue_items, &output_dir, &generated_at, &version)?;

    Ok(())
}

/// Calculates the overall and category-specific scores for the analysis.
///
/// The total score is a weighted average of:
/// - Complexity (35%)
/// - Maintainability (25%)
/// - Security (15%)
/// - Reliability (15%)
/// - Style (10%)
#[allow(clippy::too_many_lines)]
fn calculate_score(result: &AnalysisResult) -> OverallScore {
    // --- 1. Complexity (30-40% weight) ---
    // Signals: High Cyclomatic Complexity, Deep Nesting, Long Functions
    let mut complexity_penalty: f64 = 0.0;

    // Penalize based on file metrics
    for file_metric in &result.file_metrics {
        // Average complexity per function approximation (total / 1 (if no funcs)).
        // Since we don't have generic function counts per file easily here without parsing,
        // we'll rely on the specific "Quality" issues for complexity violations.

        // However, we can use the "Function too complex" issues.
        // And we can penalize huge files.
        if file_metric.sloc > 500 {
            complexity_penalty += 2.0; // Hard to understand big files
        }
    }

    // Parse complexity-related quality issues
    for issue in &result.quality {
        if issue.message.to_lowercase().contains("complex") {
            // "Function is too complex (McCabe=XX)"
            if let Some(val) = issue
                .message
                .split('=')
                .nth(1)
                .and_then(|s| s.trim_end_matches(')').parse::<f64>().ok())
            {
                if val > 10.0 {
                    complexity_penalty += (val - 10.0) * 2.0;
                }
            } else {
                complexity_penalty += 5.0; // Default penalty if parsing fails
            }
        }
        if issue.message.to_lowercase().contains("nested")
            || issue.message.to_lowercase().contains("nesting")
        {
            complexity_penalty += 5.0;
        }
        if issue.message.to_lowercase().contains("too long") {
            // "Function too long"
            complexity_penalty += 5.0;
        }
    }
    // Cap complexity penalty at 25 (not 40) to leave room for other categories
    complexity_penalty = complexity_penalty.min(25.0);

    // --- 2. Maintainability (25-30% weight) ---
    // Signals: Unused code, duplication (not yet), file size
    let mut maintainability_penalty: f64 = 0.0;

    let unused_count = result.unused_functions.len()
        + result.unused_classes.len()
        + result.unused_imports.len()
        + result.unused_variables.len()
        + result.unused_methods.len()
        + result.unused_parameters.len();

    // 2 points per unused symbol as requested
    maintainability_penalty += (unused_count as f64) * 2.0;

    // Penalize clones (3 points per duplicate)
    for clone in &result.clones {
        if clone.is_duplicate {
            maintainability_penalty += 3.0;
        }
    }

    // Cap maintainability penalty at 45 (increased from 20 to account for clones)
    maintainability_penalty = maintainability_penalty.min(45.0);

    // --- 3. Reliability / Correctness (15-20% weight) ---
    // Signals: Error handling, Exceptions
    let mut reliability_penalty: f64 = 0.0;

    for issue in &result.quality {
        let msg = issue.message.to_lowercase();
        if msg.contains("error")
            || msg.contains("exception")
            || msg.contains("panic")
            || msg.contains("unwrap")
            || msg.contains("expect")
        {
            reliability_penalty += 5.0;
        }
    }
    // Cap reliability at 15 (not 20)
    reliability_penalty = reliability_penalty.min(15.0);

    // --- 4. Security (10-15% weight) ---
    // Signals: Secrets, Danger, Taint
    let mut security_penalty: f64 = 0.0;

    // Secrets are critical
    for _ in &result.secrets {
        security_penalty += 30.0;
    }
    // Danger/Taint
    for _ in &result.danger {
        security_penalty += 15.0;
    }
    for _ in &result.taint_findings {
        security_penalty += 20.0;
    }

    // Cap security (but it can tank the score fully if needed, user said "Security penalties should be severe")
    // User said "Cap penalties" generally, but also "One hardcoded secret should tank the score".
    // So we'll let it execute, but effective score floor is 0.
    // However, for the *Category* score, we might want to cap it.
    // For main score, we subtract.

    // --- 5. Readability & Style (5-10% weight) ---
    // Signals: Other quality issues
    let mut style_penalty: f64 = 0.0;

    for issue in &result.quality {
        // Filter out things we already counted
        let msg = issue.message.to_lowercase();
        if !msg.contains("complex")
            && !msg.contains("nested")
            && !msg.contains("too long")
            && !msg.contains("error")
            && !msg.contains("exception")
            && !msg.contains("panic")
            && !msg.contains("unwrap")
        {
            // Generic lint/style issue
            style_penalty += 2.0;
        }
    }
    // Cap style at 5 (not 10)
    style_penalty = style_penalty.min(5.0);

    // Calculate category scores (used for display AND weighted average)
    let complexity_score = (100.0 - complexity_penalty).max(0.0);
    let maintainability_score = (100.0 - maintainability_penalty).max(0.0);
    let security_score = (100.0 - security_penalty.min(100.0)).max(0.0);
    let reliability_score = (100.0 - reliability_penalty).max(0.0);
    let style_score = (100.0 - style_penalty).max(0.0);

    // === WEIGHTED AVERAGE ===
    // Weights: Complexity 35%, Maintainability 25%, Security 15%, Reliability 15%, Style 10%
    let weighted_score = (complexity_score * 0.35)
        + (maintainability_score * 0.25)
        + (security_score * 0.15)
        + (reliability_score * 0.15)
        + (style_score * 0.10);

    let total = weighted_score.round() as u8;

    let grade = match total {
        90..=100 => "A",
        75..=89 => "B",
        60..=74 => "C",
        40..=59 => "D",
        _ => "F",
    }
    .to_owned();

    let grade_color = |grade: &str| -> String {
        match grade {
            "A" => "#4ade80".to_owned(), // Green-400
            "B" => "#a3e635".to_owned(), // Lime-400
            "C" => "#facc15".to_owned(), // Yellow-400
            "D" => "#fb923c".to_owned(), // Orange-400
            _ => "#f87171".to_owned(),   // Red-400
        }
    };

    let complexity_grade = match complexity_score as u8 {
        90..=100 => "A",
        75..=89 => "B",
        60..=74 => "C",
        _ => "F",
    };

    let maintainability_grade = match maintainability_score as u8 {
        90..=100 => "A",
        75..=89 => "B",
        60..=74 => "C",
        _ => "F",
    };

    let security_grade = match security_score as u8 {
        90..=100 => "A",
        75..=89 => "B",
        60..=74 => "C",
        _ => "F",
    };

    let reliability_grade = match reliability_score as u8 {
        90..=100 => "A",
        75..=89 => "B",
        60..=74 => "C",
        _ => "F",
    };

    let style_grade = match style_score as u8 {
        90..=100 => "A",
        75..=89 => "B",
        60..=74 => "C",
        _ => "F",
    };

    OverallScore {
        total_score: total,
        grade,
        categories: vec![
            CategoryScore {
                name: "Complexity".into(),
                score: complexity_score as u8,
                issue_count: 0,
                grade: complexity_grade.into(),
                color: grade_color(complexity_grade),
            },
            CategoryScore {
                name: "Maintainability".into(),
                score: maintainability_score as u8,
                issue_count: unused_count,
                grade: maintainability_grade.into(),
                color: grade_color(maintainability_grade),
            },
            CategoryScore {
                name: "Security".into(),
                score: security_score as u8,
                issue_count: result.secrets.len()
                    + result.danger.len()
                    + result.taint_findings.len(),
                grade: security_grade.into(),
                color: grade_color(security_grade),
            },
            CategoryScore {
                name: "Reliability".into(),
                score: reliability_score as u8,
                issue_count: 0,
                grade: reliability_grade.into(),
                color: grade_color(reliability_grade),
            },
            CategoryScore {
                name: "Style".into(),
                score: style_score as u8,
                issue_count: 0,
                grade: style_grade.into(),
                color: grade_color(style_grade),
            },
        ],
    }
}

fn flatten_issues(result: &AnalysisResult) -> Vec<IssueItem> {
    let mut items = Vec::new();

    // Helper closure
    let mut add = |category: &str, severity: &str, msg: String, file: String, line: usize| {
        let safe_name = file.replace(['/', '\\', ':'], "_") + ".html";
        let link = format!("files/{safe_name}#L{line}");

        items.push(IssueItem {
            category: category.to_owned(),
            severity: severity.to_owned(),
            message: msg,
            file,
            line,
            link,
            code_snippet: None,
        });
    };

    for x in &result.unused_functions {
        add(
            "Unused",
            "LOW",
            format!("Unused function: {}", x.full_name),
            x.file.to_string_lossy().to_string(),
            x.line,
        );
    }
    for x in &result.unused_classes {
        add(
            "Unused",
            "LOW",
            format!("Unused class: {}", x.full_name),
            x.file.to_string_lossy().to_string(),
            x.line,
        );
    }
    for x in &result.unused_imports {
        add(
            "Unused",
            "LOW",
            format!("Unused import: {}", x.full_name),
            x.file.to_string_lossy().to_string(),
            x.line,
        );
    }

    for x in &result.secrets {
        add(
            "Security",
            "HIGH",
            format!("Secret found: {}", x.message),
            x.file.to_string_lossy().to_string(),
            x.line,
        );
    }
    for x in &result.danger {
        add(
            "Security",
            "HIGH",
            x.message.clone(),
            x.file.to_string_lossy().to_string(),
            x.line,
        );
    }
    for x in &result.taint_findings {
        add(
            "Security",
            "CRITICAL",
            format!("Taint flow detected: {} -> {}", x.source, x.sink),
            x.file.to_string_lossy().to_string(),
            x.source_line,
        );
    }

    for x in &result.quality {
        add(
            "Quality",
            &x.severity,
            x.message.clone(),
            x.file.to_string_lossy().to_string(),
            x.line,
        );
    }
    for x in &result.parse_errors {
        add(
            "Quality",
            "HIGH",
            format!("Parse Error: {}", x.error),
            x.file.to_string_lossy().to_string(),
            0,
        );
    }

    items
}

fn segregate_issues(items: &[IssueItem]) -> (Vec<IssueItem>, Vec<IssueItem>, Vec<IssueItem>) {
    let mut unused = Vec::new();
    let mut security = Vec::new();
    let mut quality = Vec::new();

    for item in items {
        match item.category.as_str() {
            "Unused" => unused.push(item.clone()),
            "Security" => security.push(item.clone()),
            _ => quality.push(item.clone()),
        }
    }
    (unused, security, quality)
}

fn write_assets(output_dir: &Path) -> Result<()> {
    use crate::report::assets::{CHARTS_JS, PRISM_CSS, PRISM_JS, STYLE_CSS};
    fs::create_dir_all(output_dir.join("css"))?;
    fs::create_dir_all(output_dir.join("js"))?;

    fs::write(output_dir.join("css/style.css"), STYLE_CSS)?;
    fs::write(output_dir.join("js/charts.js"), CHARTS_JS)?;
    fs::write(output_dir.join("css/prism.css"), PRISM_CSS)?;
    fs::write(output_dir.join("js/prism.js"), PRISM_JS)?;

    Ok(())
}

fn generate_file_views(
    result: &AnalysisResult,
    issue_items: &[IssueItem],
    output_dir: &Path,
    generated_at: &str,
    version: &str,
) -> Result<()> {
    fs::create_dir_all(output_dir.join("files"))?;

    for file_metric in &result.file_metrics {
        let file_path_str = file_metric.file.to_string_lossy().to_string();
        let file_path = Path::new(&file_path_str);

        if file_path.exists() && file_path.is_file() {
            let code = fs::read_to_string(file_path).unwrap_or_default();
            let relative_path = file_path_str.clone();
            let safe_name = relative_path.replace(['/', '\\', ':'], "_") + ".html";

            let view = FileViewTemplate {
                version: version.to_owned(),
                relative_path: relative_path.clone(),
                code,
                // Filter issues for this file
                issues: issue_items
                    .iter()
                    .filter(|i| i.file == relative_path)
                    .cloned()
                    .collect(),
                sloc: file_metric.sloc,
                complexity: file_metric.complexity,
                mi: format!("{:.1}", file_metric.mi),
                raw_mi: file_metric.mi,
                generated_at: generated_at.to_owned(),
                root_path: "..".to_owned(),
            };

            let html = view.render()?;
            fs::write(output_dir.join("files").join(safe_name), html)?;
        }
    }
    Ok(())
}
