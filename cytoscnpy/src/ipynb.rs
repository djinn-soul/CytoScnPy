use anyhow::Result;
use nbformat::{parse_notebook, Notebook};
use std::fs;
use std::path::Path;

mod cells;
mod write;

pub use cells::assign_cells;
pub use write::write_cell_sources;

/// Extract Python code from all code cells in a notebook
///
/// # Errors
///
/// Returns an error if the file cannot be read or if parsing the notebook JSON fails.
pub fn extract_notebook_code(path: &Path, root: Option<&Path>) -> Result<String> {
    Ok(NotebookSource::read(path, root)?.code)
}

/// Separator placed between code cells when they are analyzed as one module.
const CELL_SEPARATOR: &str = "\n\n";

/// A notebook's code cells joined into the module the analyzer reads.
///
/// Analyzer byte offsets and lines index [`NotebookSource::code`]; this maps
/// them back to the cell they came from.
#[derive(Debug, Clone)]
pub struct NotebookSource {
    /// Code cells joined by blank lines.
    pub code: String,
    cells: Vec<CellSpan>,
}

/// Where one code cell sits in [`NotebookSource::code`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellSpan {
    /// Index of the cell in the notebook, counting markdown and raw cells.
    pub index: usize,
    /// Byte offset of the cell's first byte in the joined code.
    pub start: usize,
    /// Byte offset just past the cell's last byte in the joined code.
    pub end: usize,
    /// 1-based line of the cell's first line in the joined code.
    pub start_line: usize,
    /// 1-based line of the cell's last line in the joined code.
    pub end_line: usize,
}

impl CellSpan {
    /// Converts a joined-code line within this cell to a cell line.
    #[must_use]
    pub const fn relative_line(&self, line: usize) -> usize {
        line.saturating_sub(self.start_line) + 1
    }
}

impl NotebookSource {
    /// Reads the code cells of the notebook at `path`.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or if parsing the notebook JSON fails.
    pub fn read(path: &Path, root: Option<&Path>) -> Result<Self> {
        Ok(Self::from_cells(extract_notebook_cells(path, root)?))
    }

    /// Joins `(cell index, source)` pairs the way the analyzer does.
    #[must_use]
    pub fn from_cells(cells: Vec<(usize, String)>) -> Self {
        let mut code = String::new();
        let mut spans = Vec::with_capacity(cells.len());
        let mut line = 1;
        for (position, (index, source)) in cells.into_iter().enumerate() {
            if position > 0 {
                code.push_str(CELL_SEPARATOR);
                line += CELL_SEPARATOR.matches('\n').count();
            }
            let start = code.len();
            code.push_str(&source);
            let start_line = line;
            line += source.matches('\n').count();
            spans.push(CellSpan {
                index,
                start,
                end: code.len(),
                start_line,
                end_line: line,
            });
        }
        Self { code, cells: spans }
    }

    /// The cell holding the joined-code byte range `start..end`, if one does.
    #[must_use]
    pub fn cell_containing(&self, start: usize, end: usize) -> Option<CellSpan> {
        self.cells
            .iter()
            .copied()
            .find(|cell| cell.start <= start && end <= cell.end)
    }

    /// The cell holding 1-based joined-code `line`, if one does.
    #[must_use]
    pub fn cell_at_line(&self, line: usize) -> Option<CellSpan> {
        self.cells
            .iter()
            .copied()
            .find(|cell| cell.start_line <= line && line <= cell.end_line)
    }

    /// Converts a line relative to the cell at notebook index `cell` back to
    /// a line of the joined code.
    #[must_use]
    pub fn joined_line(&self, cell: usize, line: usize) -> Option<usize> {
        self.cells
            .iter()
            .find(|span| span.index == cell)
            .map(|span| span.start_line + line.saturating_sub(1))
    }

    /// Each code cell as `(notebook index, first line in the joined code)`.
    #[must_use]
    pub fn cell_starts(&self) -> Vec<(usize, usize)> {
        self.cells
            .iter()
            .map(|span| (span.index, span.start_line))
            .collect()
    }

    /// The source of the cell at notebook index `index`.
    #[must_use]
    pub fn cell_source(&self, index: usize) -> Option<&str> {
        self.cells
            .iter()
            .find(|cell| cell.index == index)
            .map(|cell| &self.code[cell.start..cell.end])
    }

    /// Splits joined-code edits `(start, end, replacement)` by cell, with
    /// offsets relative to each cell. Returns `None` when an edit spans a
    /// cell boundary, which no single cell edit can express.
    #[must_use]
    pub fn split_edits<'a>(
        &self,
        edits: impl IntoIterator<Item = (usize, usize, &'a str)>,
    ) -> Option<Vec<(CellSpan, Vec<crate::fix::Edit>)>> {
        let mut by_cell: Vec<(CellSpan, Vec<crate::fix::Edit>)> = Vec::new();
        for (start, end, replacement) in edits {
            let cell = self.cell_containing(start, end)?;
            let edit = crate::fix::Edit::new(start - cell.start, end - cell.start, replacement);
            match by_cell.iter_mut().find(|(span, _)| *span == cell) {
                Some((_, cell_edits)) => cell_edits.push(edit),
                None => by_cell.push((cell, vec![edit])),
            }
        }
        Some(by_cell)
    }

    /// Applies joined-code edits to the cells they fall in, returning each
    /// changed cell's index and new source. Every changed cell must still be
    /// valid Python on its own, since Jupyter runs cells one at a time.
    ///
    /// # Errors
    ///
    /// Returns an error if an edit spans a cell boundary or leaves a cell invalid.
    pub fn apply_edits<'a>(
        &self,
        edits: impl IntoIterator<Item = (usize, usize, &'a str)>,
    ) -> Result<Vec<(usize, String)>> {
        let by_cell = self
            .split_edits(edits)
            .ok_or_else(|| anyhow::anyhow!("A fix spans more than one notebook cell"))?;
        by_cell
            .into_iter()
            .map(|(cell, cell_edits)| {
                let mut rewriter =
                    crate::fix::ByteRangeRewriter::new(&self.code[cell.start..cell.end]);
                rewriter.add_edits(cell_edits);
                let fixed = rewriter
                    .apply_verified()
                    .map_err(|error| anyhow::anyhow!("Notebook cell {}: {error}", cell.index))?;
                Ok((cell.index, fixed))
            })
            .collect()
    }
}

/// Extract code cells with their indices for cell-level reporting
///
/// # Errors
///
/// Returns an error if the file cannot be read or if parsing the notebook JSON fails.
pub fn extract_notebook_cells(path: &Path, root: Option<&Path>) -> Result<Vec<(usize, String)>> {
    let path = crate::utils::validate_source_path(path, root)?;
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
        Notebook::V3(nb) => nb
            .worksheets
            .iter()
            .flatten()
            .flat_map(|ws| ws.cells.iter())
            .enumerate()
            .filter_map(|(idx, cell)| match cell {
                nbformat::v3::Cell::Code { input, .. } => input.as_ref().map(|i| (idx, i.join(""))),
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

#[cfg(test)]
mod tests;
