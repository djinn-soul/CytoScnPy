//! Plan, normalize and verify dead-code fixes for one parsed source file.

use super::apply_plan::{normalize_planned_edits, plan_edits, PlannedEdit};
use super::names::ModuleNames;
use super::syntax::preserve_syntax;
use crate::fix::{ByteRangeRewriter, Edit};

use anyhow::{Context, Result};
use ruff_text_size::Ranged;
use std::path::Path;

/// A parsed source file that fixes are planned against.
pub(super) struct FixSource<'a> {
    content: &'a str,
    module: ruff_python_ast::ModModule,
    module_names: ModuleNames,
    #[cfg(feature = "cst")]
    cst_mapper: Option<crate::cst::AstCstMapper>,
}

/// Normalized edits together with the source they produce, which parses.
pub(super) struct VerifiedFix {
    pub(super) planned: Vec<PlannedEdit>,
    pub(super) fixed: String,
}

impl<'a> FixSource<'a> {
    pub(super) fn parse(content: &'a str, with_cst: bool) -> Result<Self> {
        let module = ruff_python_parser::parse_module(content)?.into_syntax();
        #[cfg(not(feature = "cst"))]
        let _ = with_cst;
        Ok(Self {
            content,
            module_names: ModuleNames::new(&module),
            module,
            #[cfg(feature = "cst")]
            cst_mapper: build_cst_mapper(content, with_cst),
        })
    }

    /// Plans the removal of every item, failing unless all of them can be
    /// removed together and the result is valid Python.
    pub(super) fn plan(
        &self,
        file_path: &Path,
        items: &[(&'static str, &crate::visitor::Definition)],
    ) -> Result<VerifiedFix> {
        let planned = {
            #[cfg(feature = "cst")]
            {
                plan_edits(
                    items,
                    &self.module,
                    &self.module_names,
                    self.content,
                    self.cst_mapper.as_ref(),
                )
            }
            #[cfg(not(feature = "cst"))]
            {
                plan_edits(items, &self.module, &self.module_names, self.content)
            }
        };

        let mut planned = normalize_planned_edits(planned)
            .with_context(|| format!("Failed to plan fixes for {}", file_path.display()))?;
        anyhow::ensure!(
            !planned.is_empty(),
            "No fixes could be planned for selected definitions in {}",
            file_path.display()
        );
        anyhow::ensure!(
            planned
                .iter()
                .map(|edit| edit.removed_names.len())
                .sum::<usize>()
                == items.len(),
            "Some selected definitions could not be located in {}",
            file_path.display()
        );
        preserve_syntax(&self.module, self.content, &mut planned);
        let planned = normalize_planned_edits(planned)
            .with_context(|| format!("Failed to preserve syntax for {}", file_path.display()))?;
        let mut preview = ByteRangeRewriter::new(self.content);
        preview.add_edits(planned.iter().map(|edit| {
            Edit::new(
                edit.start_byte,
                edit.end_byte,
                edit.replacement.as_deref().unwrap_or(""),
            )
        }));
        let fixed = preview
            .apply()
            .with_context(|| format!("Failed to fix {}", file_path.display()))?;
        self.verify(&planned, &fixed).with_context(|| {
            format!(
                "Failed to fix {}: generated invalid Python",
                file_path.display()
            )
        })?;
        Ok(VerifiedFix { planned, fixed })
    }

    /// Checks that `fixed`, the source with `planned` applied, parses.
    ///
    /// Fixes are planned one finding at a time, so re-parsing the whole file
    /// for each would cost findings x file size. Only the top-level statements
    /// the edits touch, widened to whole lines, are parsed instead. That region
    /// starts and ends outside every top-level statement, where the tokenizer
    /// holds no open brackets, strings or indentation, so the file parses
    /// exactly when the region does.
    fn verify(&self, planned: &[PlannedEdit], fixed: &str) -> Result<()> {
        let region = planned
            .iter()
            .map(|edit| (edit.start_byte, edit.end_byte))
            .reduce(|(start, end), (s, e)| (start.min(s), end.max(e)))
            .map(|(start, end)| self.top_level_region(start, end));
        let checked = match region {
            Some((start, end)) => {
                // Every edit lies inside the region, so it shifts only the
                // region's end.
                let added: usize = planned
                    .iter()
                    .map(|edit| edit.replacement.as_deref().map_or(0, str::len))
                    .sum();
                let removed: usize = planned
                    .iter()
                    .map(|edit| edit.end_byte - edit.start_byte)
                    .sum();
                let fixed_end = (end + added).checked_sub(removed).unwrap_or(start);
                let text = fixed.get(start..fixed_end).unwrap_or(fixed);
                // A region that loses its final newline joins the next line.
                if end < self.content.len() && !text.is_empty() && !text.ends_with('\n') {
                    fixed
                } else {
                    text
                }
            }
            None => fixed,
        };
        ruff_python_parser::parse_module(checked)?;
        Ok(())
    }

    /// The smallest run of whole lines covering `start..end` that no
    /// top-level statement crosses.
    fn top_level_region(&self, start: usize, end: usize) -> (usize, usize) {
        let content = self.content;
        let line_start = |i: usize| content[..i].rfind('\n').map_or(0, |p| p + 1);
        let line_end = |i: usize| content[i..].find('\n').map_or(content.len(), |p| i + p + 1);
        let (mut region_start, mut region_end) = (line_start(start), line_end(end.max(start)));
        loop {
            let (mut new_start, mut new_end) = (region_start, region_end);
            for stmt in &self.module.body {
                let range = stmt.range();
                let (stmt_start, stmt_end) = (range.start().to_usize(), range.end().to_usize());
                if stmt_start < new_end && stmt_end > new_start {
                    new_start = new_start.min(line_start(stmt_start));
                    new_end = new_end.max(line_end(stmt_end));
                }
            }
            if (new_start, new_end) == (region_start, region_end) {
                return (region_start, region_end);
            }
            (region_start, region_end) = (new_start, new_end);
        }
    }
}

#[cfg(feature = "cst")]
fn build_cst_mapper(content: &str, with_cst: bool) -> Option<crate::cst::AstCstMapper> {
    use crate::cst::CstParser;

    if !with_cst {
        return None;
    }

    CstParser::new()
        .ok()
        .and_then(|mut parser| parser.parse(content).ok())
        .map(crate::cst::AstCstMapper::new)
}

#[cfg(test)]
mod tests {
    use super::{FixSource, PlannedEdit};

    /// Checking only the touched region must agree with parsing the whole
    /// fixed file.
    fn assert_matches_full_parse(content: &str, start: usize, end: usize, replacement: &str) {
        let source = FixSource::parse(content, false).unwrap();
        let planned = [PlannedEdit {
            start_byte: start,
            end_byte: end,
            replacement: Some(replacement.to_owned()),
            name: String::new(),
            removed_names: Vec::new(),
            item_type: "function",
            line: 1,
        }];
        let fixed = format!("{}{replacement}{}", &content[..start], &content[end..]);
        let full = ruff_python_parser::parse_module(&fixed).is_ok();
        assert_eq!(
            source.verify(&planned, &fixed).is_ok(),
            full,
            "region check disagrees with full parse for {fixed:?}"
        );
    }

    fn span(content: &str, text: &str) -> (usize, usize) {
        let start = content.find(text).unwrap();
        (start, start + text.len())
    }

    #[test]
    fn region_check_agrees_with_full_parse() {
        let decorated = "x = 0\n@dec\ndef f():\n    pass\ny = 1\n";
        let (start, end) = span(decorated, "def f():\n    pass\n");
        assert_matches_full_parse(decorated, start, end, "");

        let lines = "x = 1\ny = 2\n";
        assert_matches_full_parse(lines, 5, 6, "");
        let (start, end) = span(lines, "y = 2\n");
        assert_matches_full_parse(lines, start, end, "");

        let semicolons = "a = 1; b = 2\nc = 3\n";
        let (start, end) = span(semicolons, "b = 2");
        assert_matches_full_parse(semicolons, start, end, "");

        let suite = "def f():\n    x = 1\nz = 2\n";
        let (start, end) = span(suite, "    x = 1\n");
        assert_matches_full_parse(suite, start, end, "");
        assert_matches_full_parse(suite, start, end, "    pass\n");

        let brackets = "items = [\n    1,\n]\ndef g():\n    return 1\n";
        let (start, end) = span(brackets, "def g():\n    return 1\n");
        assert_matches_full_parse(brackets, start, end, "");
        let (start, end) = span(brackets, "]\n");
        assert_matches_full_parse(brackets, start, end, "");
    }
}
