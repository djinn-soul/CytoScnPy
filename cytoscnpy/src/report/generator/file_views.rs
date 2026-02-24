use crate::analyzer::AnalysisResult;
use crate::report::templates::{FileViewTemplate, IssueItem};
use anyhow::Result;
use askama::Template;
use std::fs;
use std::path::Path;

pub(super) fn generate_file_views(
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
                issues: issue_items
                    .iter()
                    .filter(|item| item.file == relative_path)
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
