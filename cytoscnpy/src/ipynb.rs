use anyhow::Result;
use nbformat::{parse_notebook, Notebook};
use std::fs;
use std::path::Path;

/// Extract Python code from all code cells in a notebook
///
/// # Errors
///
/// Returns an error if the file cannot be read or if parsing the notebook JSON fails.
pub fn extract_notebook_code(path: &Path, root: Option<&Path>) -> Result<String> {
    let path = crate::utils::validate_output_path(path, root)?;
    let notebook_json = fs::read_to_string(path)?;
    let notebook = parse_notebook(&notebook_json)?;

    let code_cells: Vec<String> = match notebook {
        Notebook::V4(nb) => nb
            .cells
            .iter()
            .filter_map(|cell| match cell {
                nbformat::v4::Cell::Code { source, .. } => Some(source.join("")),
                _ => None,
            })
            .collect(),
        Notebook::Legacy(nb) => nb
            .cells
            .iter()
            .filter_map(|cell| match cell {
                nbformat::legacy::Cell::Code { source, .. } => Some(source.join("")),
                _ => None,
            })
            .collect(),
    };

    Ok(code_cells.join("\n\n"))
}

/// Extract code cells with their indices for cell-level reporting
///
/// # Errors
///
/// Returns an error if the file cannot be read or if parsing the notebook JSON fails.
pub fn extract_notebook_cells(path: &Path, root: Option<&Path>) -> Result<Vec<(usize, String)>> {
    let path = crate::utils::validate_output_path(path, root)?;
    let notebook_json = fs::read_to_string(path)?;
    let notebook = parse_notebook(&notebook_json)?;

    let cells: Vec<(usize, String)> = match notebook {
        Notebook::V4(nb) => nb
            .cells
            .iter()
            .enumerate()
            .filter_map(|(idx, cell)| match cell {
                nbformat::v4::Cell::Code { source, .. } => Some((idx, source.join(""))),
                _ => None,
            })
            .collect(),
        Notebook::Legacy(nb) => nb
            .cells
            .iter()
            .enumerate()
            .filter_map(|(idx, cell)| match cell {
                nbformat::legacy::Cell::Code { source, .. } => Some((idx, source.join(""))),
                _ => None,
            })
            .collect(),
    };

    Ok(cells)
}
