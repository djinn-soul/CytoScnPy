use crate::analyzer::AnalysisResult;
use crate::ipynb::NotebookSource;
use crate::report::templates::{FileViewTemplate, IssueItem};
use anyhow::Result;
use askama::Template;
use rustc_hash::FxHashMap;
use std::fs;
use std::path::Path;

fn is_notebook(path: &Path) -> bool {
    path.extension().is_some_and(|ext| ext == "ipynb")
}

/// Reads a notebook by its reported path, or `None` if it cannot be parsed.
fn read_notebook(path: &Path, root: &Path) -> Option<NotebookSource> {
    NotebookSource::read(path, Some(root)).ok()
}

/// Points notebook issues at their line in the file view, which shows the
/// notebook's code cells joined together rather than the raw JSON.
pub(super) fn map_notebook_lines(issues: &mut [IssueItem], root: &Path) {
    let mut notebooks: FxHashMap<String, Option<NotebookSource>> = FxHashMap::default();
    for issue in issues {
        let Some(cell) = issue.cell else { continue };
        let notebook = notebooks
            .entry(issue.file.clone())
            .or_insert_with(|| read_notebook(Path::new(&issue.file), root));
        if let Some(line) = notebook
            .as_ref()
            .and_then(|notebook| notebook.joined_line(cell, issue.line))
        {
            issue.view_line = line;
        }
    }
}

pub(super) fn generate_file_views(
    result: &AnalysisResult,
    issue_items: &[IssueItem],
    output_dir: &Path,
    generated_at: &str,
    version: &str,
    paths: &super::paths::ReportPaths,
    root: &Path,
) -> Result<()> {
    fs::create_dir_all(output_dir.join("files"))?;

    for file_metric in &result.file_metrics {
        let file_path_str = file_metric.file.to_string_lossy().to_string();
        let file_path = Path::new(&file_path_str);

        if file_path.exists() && file_path.is_file() {
            let (code, cells) = if is_notebook(file_path) {
                match read_notebook(file_path, root) {
                    Some(notebook) => {
                        let cells = notebook.cell_starts();
                        (notebook.code, cells)
                    }
                    None => (
                        fs::read_to_string(file_path).unwrap_or_default(),
                        Vec::new(),
                    ),
                }
            } else {
                (
                    fs::read_to_string(file_path).unwrap_or_default(),
                    Vec::new(),
                )
            };
            let relative_path = file_path_str.clone();
            let safe_name = paths.filename(&relative_path)?;

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
                cells,
            };

            let html = view.render()?;
            fs::write(output_dir.join("files").join(safe_name), html)?;
        }
    }

    Ok(())
}
