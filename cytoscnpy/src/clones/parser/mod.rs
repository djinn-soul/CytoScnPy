//! Parser integration with `ruff_python_parser`.
//!
//! Extracts subtrees from Python source code for clone detection.

mod definitions;
mod expression_complex;
mod expressions;
mod extract;
mod patterns;
mod statement_misc;
mod statements;
#[cfg(test)]
mod tests;
mod types;

pub use extract::extract_subtrees;
pub use types::{CloneFingerprint, Subtree, SubtreeNode, SubtreeType};

pub(crate) fn extract_subtrees_with_min_lines(
    source: &str,
    path: &std::path::PathBuf,
    min_lines: usize,
) -> Result<Vec<Subtree>, crate::clones::CloneError> {
    extract::extract_subtrees_with_min_lines(source, path, min_lines)
}
