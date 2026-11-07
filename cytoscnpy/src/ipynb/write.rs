//! Writing fixed cell sources back into a notebook's JSON in place.

use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

/// Replaces the source of the given code cells in the notebook at `path`,
/// keeping every other byte of its JSON layout that serialization allows.
///
/// # Errors
///
/// Returns an error if the notebook cannot be read, is not nbformat 4, or
/// cannot be written.
pub fn write_cell_sources(
    path: &Path,
    root: Option<&Path>,
    cells: &[(usize, String)],
) -> Result<()> {
    let path = crate::utils::validate_source_path(path, root)?;
    let original = fs::read_to_string(&path)?;
    let fixed = splice_cell_sources(&original, cells)
        .with_context(|| format!("Failed to fix {}", path.display()))?;
    fs::write(&path, fixed)?;
    Ok(())
}

/// Replaces the `source` of the given cells in the notebook JSON text,
/// leaving every other byte as it was. Re-serializing the whole notebook
/// would reformat numbers and strings in outputs and metadata.
fn splice_cell_sources(original: &str, cells: &[(usize, String)]) -> Result<String> {
    use serde::Deserialize;
    use serde_json::value::RawValue;

    #[derive(Deserialize)]
    struct RawNotebook<'a> {
        nbformat: u64,
        #[serde(borrow)]
        cells: Vec<&'a RawValue>,
    }
    #[derive(Deserialize)]
    struct RawCell<'a> {
        #[serde(borrow)]
        source: &'a RawValue,
    }

    let notebook: RawNotebook = serde_json::from_str(original)?;
    anyhow::ensure!(
        notebook.nbformat == 4,
        "Only nbformat 4 notebooks can be fixed"
    );
    // Jupyter writes one-space indentation; follow whatever the file uses.
    let unit = original
        .lines()
        .nth(1)
        .map_or(1, |line| line.len() - line.trim_start_matches(' ').len())
        .max(1);

    let mut edits = Vec::with_capacity(cells.len());
    for (index, source) in cells {
        let cell = notebook
            .cells
            .get(*index)
            .ok_or_else(|| anyhow::anyhow!("Notebook has no cell {index}"))?;
        let raw = serde_json::from_str::<RawCell>(cell.get())?.source.get();
        // The raw value borrows from `original`, so its address gives its offset.
        let start = raw.as_ptr() as usize - original.as_ptr() as usize;
        let value = source_json(original, start, raw, source, unit)?;
        edits.push((start, start + raw.len(), value));
    }

    edits.sort_by_key(|(start, _, _)| *start);
    let mut out = String::with_capacity(original.len());
    let mut copied = 0;
    for (start, end, value) in edits {
        out.push_str(&original[copied..start]);
        out.push_str(&value);
        copied = end;
    }
    out.push_str(&original[copied..]);
    Ok(out)
}

/// Encodes a cell source the way the cell already stores it: one string, or
/// a list of lines laid out like the existing list.
fn source_json(
    original: &str,
    start: usize,
    raw: &str,
    source: &str,
    unit: usize,
) -> Result<String> {
    if raw.starts_with('"') {
        return Ok(serde_json::to_string(source)?);
    }
    let lines: Vec<&str> = source.split_inclusive('\n').collect();
    if lines.is_empty() {
        return Ok("[]".to_owned());
    }
    if !raw.contains('\n') {
        return Ok(serde_json::to_string(&lines)?);
    }
    // Elements sit one level deeper than the line holding `"source": [`.
    let line_start = original[..start].rfind('\n').map_or(0, |i| i + 1);
    let key_indent: String = original[line_start..start]
        .chars()
        .take_while(|c| *c == ' ' || *c == '\t')
        .collect();
    let item_indent = format!("{key_indent}{}", " ".repeat(unit));
    let mut out = String::from("[\n");
    for (i, line) in lines.iter().enumerate() {
        if i > 0 {
            out.push_str(",\n");
        }
        out.push_str(&item_indent);
        out.push_str(&serde_json::to_string(line)?);
    }
    out.push('\n');
    out.push_str(&key_indent);
    out.push(']');
    Ok(out)
}
