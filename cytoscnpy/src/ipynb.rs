use anyhow::{Context, Result};
use nbformat::{parse_notebook, Notebook};
use std::fs;
use std::path::Path;

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

/// Reports notebook findings at cell level (`--ipynb-cells`): records each
/// finding's cell and makes its lines count from the start of that cell, as
/// Jupyter and editors number them. Byte offsets keep indexing the joined
/// code. A finding on no cell's line, such as a file-level finding on line 0,
/// keeps its line and gets no cell.
pub fn assign_cells(result: &mut crate::analyzer::AnalysisResult, root: Option<&Path>) {
    let mut notebooks = Notebooks {
        root,
        sources: rustc_hash::FxHashMap::default(),
    };
    let definitions = result
        .unused_functions
        .iter_mut()
        .chain(&mut result.unused_methods)
        .chain(&mut result.unused_classes)
        .chain(&mut result.unused_imports)
        .chain(&mut result.unused_variables)
        .chain(&mut result.unused_parameters);
    for def in definitions {
        let Some(cell) = notebooks
            .get(&def.file)
            .and_then(|notebook| notebook.cell_containing(def.start_byte, def.start_byte))
        else {
            continue;
        };
        def.cell_number = Some(cell.index);
        def.line = cell.relative_line(def.line);
        def.end_line = cell.relative_line(def.end_line);
    }
    for finding in result.danger.iter_mut().chain(&mut result.quality) {
        finding.cell_number = notebooks.place(&finding.file, &mut finding.line);
    }
    for finding in &mut result.secrets {
        finding.cell_number = notebooks.place(&finding.file, &mut finding.line);
    }
    for finding in &mut result.taint_findings {
        finding.cell_number = notebooks.place(&finding.file, &mut finding.sink_line);
        finding.source_cell_number = notebooks.place(&finding.file, &mut finding.source_line);
    }
}

/// Notebooks read on demand while placing findings in cells.
struct Notebooks<'a> {
    root: Option<&'a Path>,
    sources: rustc_hash::FxHashMap<std::path::PathBuf, Option<NotebookSource>>,
}

impl Notebooks<'_> {
    fn get(&mut self, file: &Path) -> Option<&NotebookSource> {
        if !file.extension().is_some_and(|ext| ext == "ipynb") {
            return None;
        }
        let root = self.root;
        self.sources
            .entry(file.to_path_buf())
            .or_insert_with(|| NotebookSource::read(file, root).ok())
            .as_ref()
    }

    /// Returns the cell holding joined-code `line`, making `line` relative to it.
    fn place(&mut self, file: &Path, line: &mut usize) -> Option<usize> {
        let cell = self.get(file)?.cell_at_line(*line)?;
        *line = cell.relative_line(*line);
        Some(cell.index)
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
mod tests {
    use super::NotebookSource;

    fn source() -> NotebookSource {
        NotebookSource::from_cells(vec![
            (1, "import os\nx = 1".to_owned()),
            (3, "y = 2\n".to_owned()),
        ])
    }

    #[test]
    fn joins_cells_and_maps_offsets_back() {
        let notebook = source();
        assert_eq!(notebook.code, "import os\nx = 1\n\ny = 2\n");
        let second = notebook.cell_containing(17, 18).unwrap();
        assert_eq!((second.index, second.start, second.start_line), (3, 17, 4));
        assert_eq!(notebook.cell_source(3), Some("y = 2\n"));
        // The separator belongs to no cell.
        assert!(notebook.cell_containing(15, 17).is_none());
        assert_eq!(notebook.cell_at_line(2).map(|cell| cell.index), Some(1));
        assert_eq!(notebook.cell_at_line(4).map(|cell| cell.index), Some(3));
        // The blank separator line and line 0 belong to no cell.
        assert!(notebook.cell_at_line(3).is_none());
        assert!(notebook.cell_at_line(0).is_none());
    }

    #[test]
    fn applies_edits_per_cell() {
        let notebook = source();
        let cells = notebook
            .apply_edits([(10, 11, "_"), (17, 18, "_")])
            .unwrap();
        assert_eq!(
            cells,
            vec![
                (1, "import os\n_ = 1".to_owned()),
                (3, "_ = 2\n".to_owned())
            ]
        );
        assert!(notebook.apply_edits([(10, 20, "")]).is_err());
    }
}
