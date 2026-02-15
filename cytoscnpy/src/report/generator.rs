mod assets;
mod file_views;
mod issues;
mod scoring;

use crate::analyzer::AnalysisResult;
use crate::report::templates::{
    CloneItem, ClonesTemplate, DashboardTemplate, FileMetricsView, FilesTemplate, IssuesTemplate,
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
pub fn generate_report(result: &AnalysisResult, root: &Path, output_dir: &Path) -> Result<()> {
    let output_dir = crate::utils::validate_output_path(output_dir, Some(root))?;

    if !output_dir.exists() {
        fs::create_dir_all(&output_dir)?;
    }

    let score = scoring::calculate_score(result);
    let generated_at = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let version = env!("CARGO_PKG_VERSION").to_owned();

    let issue_items = issues::flatten_issues(result);
    let file_metrics_view = build_file_metrics_view(result);

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
        halstead_view: scoring::build_halstead_view(result),
        generated_at: generated_at.clone(),
        version: version.clone(),
        root_path: ".".to_owned(),
    };
    fs::write(output_dir.join("index.html"), dashboard.render()?)?;

    let (unused, security, quality) = issues::segregate_issues(&issue_items);
    let issues_page = IssuesTemplate {
        unused_code: unused,
        security,
        quality,
        generated_at: generated_at.clone(),
        version: version.clone(),
        root_path: ".".to_owned(),
    };
    fs::write(output_dir.join("issues.html"), issues_page.render()?)?;

    let files_page = FilesTemplate {
        file_metrics: file_metrics_view,
        average_mi: format!("{:.1}", result.analysis_summary.average_mi),
        average_mi_color,
        version: version.clone(),
        generated_at: generated_at.clone(),
        root_path: ".".to_owned(),
    };
    fs::write(output_dir.join("files.html"), files_page.render()?)?;

    let clones_page = ClonesTemplate {
        clones: build_clone_items(result),
        generated_at: generated_at.clone(),
        version: version.clone(),
        root_path: ".".to_owned(),
    };
    fs::write(output_dir.join("clones.html"), clones_page.render()?)?;

    assets::write_assets(&output_dir)?;
    file_views::generate_file_views(result, &issue_items, &output_dir, &generated_at, &version)?;

    Ok(())
}

fn build_file_metrics_view(result: &AnalysisResult) -> Vec<FileMetricsView> {
    result
        .file_metrics
        .iter()
        .map(|file_metric| FileMetricsView {
            file: file_metric.file.to_string_lossy().to_string(),
            sloc: file_metric.sloc,
            complexity: file_metric.complexity,
            raw_mi: file_metric.mi,
            mi: format!("{:.1}", file_metric.mi),
            total_issues: file_metric.total_issues,
            link: format!(
                "files/{}.html",
                file_metric
                    .file
                    .to_string_lossy()
                    .replace(['/', '\\', ':'], "_")
            ),
        })
        .collect()
}

fn build_clone_items(result: &AnalysisResult) -> Vec<CloneItem> {
    result
        .clones
        .iter()
        .filter(|clone| clone.is_duplicate)
        .map(|clone| {
            let safe_file = clone.file.to_string_lossy().replace(['/', '\\', ':'], "_") + ".html";
            let safe_related = clone
                .related_clone
                .file
                .to_string_lossy()
                .replace(['/', '\\', ':'], "_")
                + ".html";

            CloneItem {
                similarity: clone.similarity,
                clone_type: clone.clone_type.display_name().to_owned(),
                name: clone
                    .name
                    .clone()
                    .unwrap_or_else(|| "<anonymous>".to_owned()),
                file: clone.file.to_string_lossy().to_string(),
                line: clone.line,
                link: format!("files/{}#L{}", safe_file, clone.line),
                related_file: clone.related_clone.file.to_string_lossy().to_string(),
                related_line: clone.related_clone.line,
                related_link: format!("files/{}#L{}", safe_related, clone.related_clone.line),
            }
        })
        .collect()
}
