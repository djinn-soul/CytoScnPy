//! Detection of deeply nested callbacks, closures, and control structures.

use std::path::Path;

use super::types::{AntiPatternKind, AntiPatternMatch};

const MAX_SNIPPET: usize = 120;
const MIN_NESTING_SPACES: usize = 16;

/// Detects deeply nested callbacks, closures, and control structures.
#[must_use]
pub fn detect_nested_callbacks(source: &str, file: &Path) -> Vec<AntiPatternMatch> {
    let mut matches = Vec::new();

    for (idx, line) in source.lines().enumerate() {
        let line_num = idx + 1;
        let indent = count_indentation(line);
        if indent < MIN_NESTING_SPACES {
            continue;
        }

        let trimmed = line.trim_start();
        if trimmed.starts_with('#') || trimmed.is_empty() {
            continue;
        }

        if is_callback_or_control_structure(trimmed) {
            matches.push(AntiPatternMatch {
                file: file.to_path_buf(),
                line: line_num,
                column: indent + 1,
                kind: AntiPatternKind::DeeplyNestedCallback,
                pattern_name: AntiPatternKind::DeeplyNestedCallback.as_str().to_owned(),
                description: AntiPatternKind::DeeplyNestedCallback
                    .description()
                    .to_owned(),
                snippet: trimmed.chars().take(MAX_SNIPPET).collect(),
            });
        }
    }

    matches
}

fn count_indentation(line: &str) -> usize {
    let mut count = 0;
    for ch in line.chars() {
        match ch {
            ' ' => count += 1,
            '\t' => count += 4,
            _ => break,
        }
    }
    count
}

fn is_callback_or_control_structure(trimmed: &str) -> bool {
    trimmed.starts_with("if ")
        || trimmed.starts_with("elif ")
        || trimmed.starts_with("for ")
        || trimmed.starts_with("while ")
        || trimmed.starts_with("def ")
        || trimmed.starts_with("async def ")
        || trimmed.starts_with("lambda ")
        || trimmed.starts_with("async for ")
        || trimmed.starts_with("async with ")
        || trimmed.starts_with("await ")
        || trimmed.starts_with(".then")
        || trimmed.starts_with(".catch")
        || trimmed.contains(".add_done_callback")
}
