//! Parser integration with `ruff_python_parser`.
//!
//! Extracts subtrees from Python source code for clone detection.

mod expressions;
mod extract;
mod statements;
mod tests;
mod types;

pub use extract::extract_subtrees;
pub use types::{CloneFingerprint, Subtree, SubtreeNode, SubtreeType};
