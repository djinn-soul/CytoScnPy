use super::ReportTheme;
use anyhow::Result;
use std::fs;
use std::path::Path;

pub(super) fn write_assets(output_dir: &Path, theme: ReportTheme) -> Result<()> {
    use crate::report::assets::{
        PRISM_CSS, PRISM_JS, STYLE_CSS, THEME_DEFAULT_PLACEHOLDER, THEME_JS,
    };

    fs::create_dir_all(output_dir.join("css"))?;
    fs::create_dir_all(output_dir.join("js"))?;

    fs::write(output_dir.join("css/style.css"), STYLE_CSS)?;
    fs::write(output_dir.join("css/prism.css"), PRISM_CSS)?;
    fs::write(
        output_dir.join("js/theme.js"),
        THEME_JS.replace(THEME_DEFAULT_PLACEHOLDER, theme.as_str()),
    )?;
    fs::write(output_dir.join("js/prism.js"), PRISM_JS)?;
    Ok(())
}
