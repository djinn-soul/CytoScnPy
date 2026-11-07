//! Placing analyzer findings in the notebook cells they came from.

use std::path::Path;

use super::NotebookSource;

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
