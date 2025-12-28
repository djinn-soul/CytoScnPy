//! Comment extraction from Python source using CST.
//!
//! Tree-sitter captures comments as explicit nodes, making extraction reliable.

use super::parser::{CstNode, CstTree};

/// A comment extracted from source code
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Comment {
    /// The comment text (including `#` prefix)
    pub text: String,
    /// Start byte offset
    pub start_byte: usize,
    /// End byte offset
    pub end_byte: usize,
    /// Line number (1-indexed for consistency with Ruff)
    pub line: usize,
    /// Whether this comment is inline (after code on same line)
    pub is_inline: bool,
}

impl Comment {
    /// Check if this comment is immediately before the given byte position
    /// (within the same or previous line, no code between)
    #[must_use]
    pub fn is_attached_before(&self, byte_pos: usize, source: &str) -> bool {
        if self.end_byte >= byte_pos {
            return false;
        }

        // Check if only whitespace between comment end and target position
        let between = &source[self.end_byte..byte_pos];
        between.chars().all(char::is_whitespace)
    }
}

/// Extract all comments from a CST tree
#[must_use]
pub fn extract_comments(tree: &CstTree) -> Vec<Comment> {
    let mut comments = Vec::new();
    extract_comments_recursive(&tree.root, &tree.source, &mut comments);
    comments
}

fn extract_comments_recursive(node: &CstNode, source: &str, comments: &mut Vec<Comment>) {
    if node.kind == "comment" {
        let text = source[node.start_byte..node.end_byte].to_string();
        let line = node.start_point.row + 1; // Convert to 1-indexed

        // Determine if inline by checking if there's code before on same line
        let line_start = source[..node.start_byte]
            .rfind('\n')
            .map_or(0, |pos| pos + 1);
        let before_comment = &source[line_start..node.start_byte];
        let is_inline = before_comment.chars().any(|c| !c.is_whitespace());

        comments.push(Comment {
            text,
            start_byte: node.start_byte,
            end_byte: node.end_byte,
            line,
            is_inline,
        });
    }

    for child in &node.children {
        extract_comments_recursive(child, source, comments);
    }
}

/// Find comments that are associated with a definition at the given range
///
/// Returns comments that are:
/// - Immediately before the definition (no code between)
/// - Inline with the definition signature
#[must_use]
pub fn find_associated_comments(tree: &CstTree, def_start: usize, def_end: usize) -> Vec<Comment> {
    let all_comments = extract_comments(tree);

    all_comments
        .into_iter()
        .filter(|c| {
            // Include comments immediately before
            c.is_attached_before(def_start, &tree.source)
            // Or inline comments within the definition
            || (c.start_byte >= def_start && c.end_byte <= def_end)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cst::parser::CstParser;

    #[test]
    fn test_extract_comments() {
        let source = r"# Module comment
def foo():  # inline comment
    # body comment
    pass
# After function
";
        let mut parser = CstParser::new().unwrap();
        let tree = parser.parse(source).unwrap();
        let comments = extract_comments(&tree);

        assert_eq!(comments.len(), 4);
        assert!(comments[0].text.contains("Module comment"));
        assert!(!comments[0].is_inline);

        assert!(comments[1].text.contains("inline comment"));
        assert!(comments[1].is_inline);

        assert!(comments[2].text.contains("body comment"));
        assert!(!comments[2].is_inline);
    }

    #[test]
    fn test_associated_comments() {
        let source = r"# Associated with foo
@decorator
def foo():
    pass

# Not associated
def bar():
    pass
";
        let mut parser = CstParser::new().unwrap();
        let tree = parser.parse(source).unwrap();

        // Find the decorator start (first non-comment code)
        let decorator_start = source.find('@').unwrap();
        let foo_end = source.find("pass").unwrap() + 4;

        let associated = find_associated_comments(&tree, decorator_start, foo_end);
        assert_eq!(associated.len(), 1);
        assert!(associated[0].text.contains("Associated with foo"));
    }
}
