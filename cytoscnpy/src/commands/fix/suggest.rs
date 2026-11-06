//! Replace the analyzer's raw fix ranges with fixes verified by the `--fix` planner.
//!
//! The visitor records each definition's byte range, but deleting that range
//! alone breaks Python: an alias leaves `import , sys`, a variable leaves
//! ` = 1`, and the last statement of a suite leaves it empty. Editors apply
//! the suggested fix verbatim, so every suggestion must parse once applied.
//!
//! Notebook fixes are planned against the joined code cells, then expressed
//! relative to the finding's cell: editors edit cells, not the JSON file.

use super::verified::{FixSource, VerifiedFix};
use crate::analyzer::types::{FixEdit, FixSuggestion};
use crate::analyzer::AnalysisResult;
use crate::ipynb::NotebookSource;
use crate::visitor::Definition;

use rayon::prelude::*;
use rustc_hash::FxHashMap;
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

const FIXABLE_KINDS: [&str; 5] = ["function", "method", "class", "import", "variable"];

/// Source text that fixes are planned against.
enum SourceText {
    Python(String),
    Notebook(NotebookSource),
}

impl SourceText {
    fn code(&self) -> &str {
        match self {
            Self::Python(code) => code,
            Self::Notebook(notebook) => &notebook.code,
        }
    }
}

/// Verifies the fix suggestions in `result` against the analyzed files on
/// disk. Notebooks are read relative to `root`, as the analyzer read them.
pub fn verify_fix_suggestions(result: &mut AnalysisResult, root: Option<&Path>) {
    verify_with_sources(result, |path| {
        if path.extension().is_some_and(|ext| ext == "ipynb") {
            NotebookSource::read(path, root)
                .ok()
                .map(SourceText::Notebook)
        } else {
            std::fs::read_to_string(path).ok().map(SourceText::Python)
        }
    });
}

/// Verifies the fix suggestions in `result` against the Python sources `read`
/// returns.
///
/// Each fixable finding keeps a suggestion only when applying it alone yields
/// valid Python. One finding per rule, and one per file (per cell in a
/// notebook), also carries the edits that fix all of them together. Unused
/// parameters get no suggestion: removing one changes a signature that
/// callers depend on.
pub fn verify_fix_suggestions_with<F>(result: &mut AnalysisResult, read: F)
where
    F: Fn(&Path) -> Option<String> + Sync,
{
    verify_with_sources(result, |path| read(path).map(SourceText::Python));
}

fn verify_with_sources<F>(result: &mut AnalysisResult, read: F)
where
    F: Fn(&Path) -> Option<SourceText> + Sync,
{
    for def in &mut result.unused_parameters {
        def.fix = None;
    }

    let fixes: Vec<(Vec<(usize, usize)>, Vec<Option<FixSuggestion>>)> = {
        let lists: [&[Definition]; 5] = [
            &result.unused_functions,
            &result.unused_methods,
            &result.unused_classes,
            &result.unused_imports,
            &result.unused_variables,
        ];
        let mut by_file: FxHashMap<&Arc<std::path::PathBuf>, Vec<(usize, usize)>> =
            FxHashMap::default();
        for (kind, defs) in lists.iter().enumerate() {
            for (index, def) in defs.iter().enumerate() {
                by_file.entry(&def.file).or_default().push((kind, index));
            }
        }
        by_file
            .into_par_iter()
            .map(|(path, indices)| {
                let items: Vec<_> = indices
                    .iter()
                    .map(|&(kind, index)| (FIXABLE_KINDS[kind], &lists[kind][index]))
                    .collect();
                let fixes = read(path).map_or_else(
                    || vec![None; items.len()],
                    |text| file_fixes(path, &text, &items),
                );
                (indices, fixes)
            })
            .collect()
    };

    for (indices, file_fixes) in fixes {
        for ((kind, index), fix) in indices.into_iter().zip(file_fixes) {
            let defs = match kind {
                0 => &mut result.unused_functions,
                1 => &mut result.unused_methods,
                2 => &mut result.unused_classes,
                3 => &mut result.unused_imports,
                _ => &mut result.unused_variables,
            };
            defs[index].fix = fix.map(Box::new);
        }
    }
}

fn file_fixes(
    path: &Path,
    text: &SourceText,
    items: &[(&'static str, &Definition)],
) -> Vec<Option<FixSuggestion>> {
    let content = text.code();
    let Ok(source) = FixSource::parse(content, true) else {
        return vec![None; items.len()];
    };
    let notebook = match text {
        SourceText::Python(_) => None,
        SourceText::Notebook(notebook) => Some(notebook),
    };
    // The cell each item sits in; a Python file is a single scope.
    let scopes: Vec<Option<usize>> = items
        .iter()
        .map(|(_, def)| {
            notebook
                .and_then(|notebook| notebook.cell_containing(def.start_byte, def.start_byte))
                .map(|cell| cell.index)
        })
        .collect();
    if notebook.is_some() && scopes.iter().any(Option::is_none) {
        return vec![None; items.len()];
    }
    let localize = |edits: Vec<FixEdit>, scope: Option<usize>| match notebook {
        None => Some(edits),
        Some(notebook) => cell_edits(notebook, &edits, scope?),
    };

    let mut fixes: Vec<Option<FixSuggestion>> = items
        .iter()
        .zip(&scopes)
        .map(|(item, &scope)| {
            let fix = source.plan(path, std::slice::from_ref(item)).ok()?;
            let fix = single_fix(content, &fix);
            let edit = FixEdit {
                end_byte: fix.end_byte,
                replacement: fix.replacement,
                start_byte: fix.start_byte,
            };
            let [edit] = <[FixEdit; 1]>::try_from(localize(vec![edit], scope)?).ok()?;
            Some(FixSuggestion {
                cell_number: scope,
                ..FixSuggestion::replacement(edit.start_byte, edit.end_byte, edit.replacement)
            })
        })
        .collect();

    let mut by_rule: BTreeMap<(&str, Option<usize>), Vec<usize>> = BTreeMap::new();
    let mut by_scope: BTreeMap<Option<usize>, Vec<usize>> = BTreeMap::new();
    for index in (0..items.len()).filter(|&i| fixes[i].is_some()) {
        by_rule
            .entry((items[index].0, scopes[index]))
            .or_default()
            .push(index);
        by_scope.entry(scopes[index]).or_default().push(index);
    }
    for ((_, scope), group) in by_rule {
        if let Some(edits) =
            combined_edits(&source, path, items, &group).and_then(|edits| localize(edits, scope))
        {
            if let Some(fix) = &mut fixes[group[0]] {
                fix.rule_fix_all = edits;
            }
        }
    }
    for (scope, group) in by_scope {
        if let Some(edits) =
            combined_edits(&source, path, items, &group).and_then(|edits| localize(edits, scope))
        {
            if let Some(fix) = &mut fixes[group[0]] {
                fix.file_fix_all = edits;
            }
        }
    }
    fixes
}

/// Re-expresses joined-code edits relative to `cell`, when they all fall
/// inside it and leave it valid Python on its own.
fn cell_edits(notebook: &NotebookSource, edits: &[FixEdit], cell: usize) -> Option<Vec<FixEdit>> {
    let joined = || {
        edits
            .iter()
            .map(|edit| (edit.start_byte, edit.end_byte, edit.replacement.as_str()))
    };
    let [(span, cell_edits)] = <[_; 1]>::try_from(notebook.split_edits(joined())?).ok()?;
    if span.index != cell {
        return None;
    }
    notebook.apply_edits(joined()).ok()?;
    Some(
        cell_edits
            .into_iter()
            .map(|edit| FixEdit {
                end_byte: edit.end_byte,
                replacement: edit.replacement,
                start_byte: edit.start_byte,
            })
            .collect(),
    )
}

/// Collapses a verified fix into one replacement spanning all of its edits.
fn single_fix(content: &str, fix: &VerifiedFix) -> FixSuggestion {
    let start = fix
        .planned
        .iter()
        .map(|edit| edit.start_byte)
        .min()
        .unwrap_or(0);
    let end = fix
        .planned
        .iter()
        .map(|edit| edit.end_byte)
        .max()
        .unwrap_or(0);
    // Text outside the edited span is unchanged, so the span's replacement is
    // whatever the fixed source holds between the same unchanged ends.
    let replacement = fix.fixed[start..fix.fixed.len() - (content.len() - end)].to_owned();
    FixSuggestion::replacement(start, end, replacement)
}

/// Edits fixing every item in `group` together, when there are several and
/// they can all be fixed at once.
fn combined_edits(
    source: &FixSource<'_>,
    path: &Path,
    items: &[(&'static str, &Definition)],
    group: &[usize],
) -> Option<Vec<FixEdit>> {
    if group.len() < 2 {
        return None;
    }
    let selected: Vec<_> = group.iter().map(|&i| items[i]).collect();
    let fix = source.plan(path, &selected).ok()?;
    Some(
        fix.planned
            .into_iter()
            .map(|edit| FixEdit {
                end_byte: edit.end_byte,
                replacement: edit.replacement.unwrap_or_default(),
                start_byte: edit.start_byte,
            })
            .collect(),
    )
}
