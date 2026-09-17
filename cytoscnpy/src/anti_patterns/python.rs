//! Python analysis coordinator for magic numbers and deeply nested callbacks.

use std::path::Path;

use super::callbacks::detect_nested_callbacks;
use super::magic::detect_magic_numbers;
use super::types::AntiPatternMatch;
use crate::utils::LineIndex;

/// Analyzes Python source code for magic numbers and deeply nested callbacks.
#[must_use]
pub fn detect_anti_patterns(source: &str, file: &Path) -> Vec<AntiPatternMatch> {
    let line_index = LineIndex::new(source);

    // 1. Detect magic numbers using AST
    let mut matches = detect_magic_numbers(source, file, &line_index);

    // 2. Detect deeply nested callbacks and control structures
    let mut nested = detect_nested_callbacks(source, file);
    matches.append(&mut nested);

    // Sort deterministically by line, then column
    matches.sort_by(|a, b| a.line.cmp(&b.line).then_with(|| a.column.cmp(&b.column)));
    matches
}
