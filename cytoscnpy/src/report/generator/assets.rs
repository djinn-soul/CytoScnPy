use anyhow::Result;
use std::fs;
use std::path::Path;

pub(super) fn write_assets(output_dir: &Path) -> Result<()> {
    use crate::report::assets::{CHARTS_JS, PRISM_CSS, PRISM_JS, STYLE_CSS};

    fs::create_dir_all(output_dir.join("css"))?;
    fs::create_dir_all(output_dir.join("js"))?;

    fs::write(output_dir.join("css/style.css"), STYLE_CSS)?;
    fs::write(output_dir.join("js/charts.js"), CHARTS_JS)?;
    fs::write(output_dir.join("css/prism.css"), PRISM_CSS)?;
    fs::write(output_dir.join("js/prism.js"), PRISM_JS)?;
    Ok(())
}
