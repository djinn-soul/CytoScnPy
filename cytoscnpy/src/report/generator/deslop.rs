use super::ReportTheme;
use crate::report::templates::{DeslopTemplate, DeslopView};
use anyhow::Result;
use askama::Template;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

const PAGE: &str = "deslop.html";
/// The last `cytoscnpy deslop --html` result, kept so the main report can
/// re-render the page with its full navigation.
const DATA: &str = "deslop.json";

#[derive(Serialize, Deserialize)]
struct StoredDeslop {
    generated_at: String,
    version: String,
    view: DeslopView,
}

/// Writes the Slop Index page as part of the main report: from the last
/// `cytoscnpy deslop --html` result if there is one, otherwise a page that
/// invites running it.
pub(super) fn write_report_page(
    output_dir: &Path,
    generated_at: &str,
    version: &str,
) -> Result<()> {
    let stored = fs::read_to_string(output_dir.join(DATA))
        .ok()
        .and_then(|data| serde_json::from_str::<StoredDeslop>(&data).ok());
    let page = match stored {
        Some(stored) => DeslopTemplate {
            deslop: Some(stored.view),
            standalone: false,
            generated_at: stored.generated_at,
            version: stored.version,
            root_path: ".".to_owned(),
            theme: None,
        },
        None => DeslopTemplate {
            deslop: None,
            standalone: false,
            generated_at: generated_at.to_owned(),
            version: version.to_owned(),
            root_path: ".".to_owned(),
            theme: None,
        },
    };
    fs::write(output_dir.join(PAGE), page.render()?)?;
    Ok(())
}

/// Writes the Slop Index page into the HTML report directory.
///
/// When no main report exists there yet, the page is written on its own with
/// the report's stylesheets, and its navigation links only to itself. A later
/// main report re-renders it with full navigation. `theme` sets the theme the
/// page opens in; without it the page follows the report's default.
///
/// # Errors
///
/// Returns an error if directory creation, file I/O, or template rendering fails.
pub fn generate_deslop_report(
    view: DeslopView,
    root: &Path,
    output_dir: &Path,
    theme: Option<ReportTheme>,
) -> Result<std::path::PathBuf> {
    let output_dir = crate::utils::validate_output_path(output_dir, Some(root))?;
    fs::create_dir_all(&output_dir)?;

    let standalone = !output_dir.join("index.html").exists();
    if standalone || !output_dir.join("css/style.css").exists() {
        super::assets::write_assets(&output_dir, theme.unwrap_or_default())?;
    }

    let stored = StoredDeslop {
        generated_at: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        version: env!("CARGO_PKG_VERSION").to_owned(),
        view,
    };
    fs::write(output_dir.join(DATA), serde_json::to_string(&stored)?)?;

    let page = DeslopTemplate {
        deslop: Some(stored.view),
        standalone,
        generated_at: stored.generated_at,
        version: stored.version,
        root_path: ".".to_owned(),
        theme: theme.map(ReportTheme::as_str),
    };
    let path = output_dir.join(PAGE);
    fs::write(&path, page.render()?)?;
    Ok(path)
}
