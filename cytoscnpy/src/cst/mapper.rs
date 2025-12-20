//! AST ↔ CST mapping for precise source location resolution.
//!
//! Maps between Ruff AST nodes (semantic) and Tree-sitter CST nodes (byte-precise).
//!
//! # Design Principles
//!
//! - **Byte-range anchored**: Find CST nodes by byte offset, not structural matching
//! - **Decorators belong to definitions**: Expand ranges to include decorators
//! - **Docstrings are code**: Don't expand for docstrings (they're literals)
//! - **Comments: contiguous only**: Include only if immediately adjacent

use super::comments::{extract_comments, find_associated_comments, Comment};
use super::parser::{CstNode, CstTree};

/// Maps between Ruff AST and Tree-sitter CST using byte ranges
pub struct AstCstMapper {
    tree: CstTree,
}

impl AstCstMapper {
    /// Create a new mapper from a parsed CST tree
    #[must_use]
    pub fn new(tree: CstTree) -> Self {
        Self { tree }
    }

    /// Get the underlying CST tree
    #[must_use]
    pub fn tree(&self) -> &CstTree {
        &self.tree
    }

    /// Get the source code
    #[must_use]
    pub fn source(&self) -> &str {
        &self.tree.source
    }

    /// Find the smallest CST node that covers the given byte range
    #[must_use]
    pub fn find_covering_node(&self, start: usize, end: usize) -> Option<&CstNode> {
        self.tree.root.find_smallest_covering(start, end)
    }

    /// Expand range backward to include decorators
    ///
    /// Decorators belong to the definition they decorate.
    #[must_use]
    pub fn expand_for_decorators(&self, start: usize, end: usize) -> (usize, usize) {
        // Find the definition node
        if let Some(def_node) = self.find_covering_node(start, end) {
            // Look for decorated_definition parent
            if def_node.kind == "function_definition" || def_node.kind == "class_definition" {
                // Walk up to find decorated_definition
                for parent_candidate in self.find_decorated_definitions() {
                    if parent_candidate.start_byte <= start && parent_candidate.end_byte >= end {
                        return (parent_candidate.start_byte, parent_candidate.end_byte);
                    }
                }
            }
        }
        (start, end)
    }

    /// Find all decorated_definition nodes
    fn find_decorated_definitions(&self) -> Vec<&CstNode> {
        self.tree.root.find_by_kind("decorated_definition")
    }

    /// Get precise byte range for a definition, including decorators
    ///
    /// Policy:
    /// - Decorators: included (belong to definition)
    /// - Docstrings: NOT expanded (they're code, already in AST range)
    /// - Comments before decorators: included only if contiguous
    #[must_use]
    pub fn precise_range_for_def(&self, ast_start: usize, ast_end: usize) -> (usize, usize) {
        let (with_decorators_start, with_decorators_end) =
            self.expand_for_decorators(ast_start, ast_end);

        // Check for contiguous comments before
        let comments = extract_comments(&self.tree);
        let mut final_start = with_decorators_start;

        for comment in comments.iter().rev() {
            if comment.is_attached_before(final_start, &self.tree.source) {
                final_start = comment.start_byte;
            } else if comment.end_byte < final_start {
                // Hit non-contiguous gap, stop
                break;
            }
        }

        (final_start, with_decorators_end)
    }

    /// Extract source slice for a definition, preserving comments
    #[must_use]
    pub fn slice_for_def(&self, ast_start: usize, ast_end: usize) -> &str {
        let (start, end) = self.precise_range_for_def(ast_start, ast_end);
        self.tree.slice(start, end)
    }

    /// Get comments associated with a definition
    #[must_use]
    pub fn comments_for_def(&self, ast_start: usize, ast_end: usize) -> Vec<Comment> {
        let (start, end) = self.precise_range_for_def(ast_start, ast_end);
        find_associated_comments(&self.tree, start, end)
    }

    /// Check if the definition has interleaved comments (within the body)
    #[must_use]
    pub fn has_interleaved_comments(&self, ast_start: usize, ast_end: usize) -> bool {
        let comments = self.comments_for_def(ast_start, ast_end);
        comments
            .iter()
            .any(|c| c.start_byte > ast_start && c.end_byte < ast_end && !c.is_inline)
    }

    /// Check if the definition is deeply nested (>2 levels)
    #[must_use]
    pub fn is_deeply_nested(&self, ast_start: usize, ast_end: usize) -> bool {
        if let Some(node) = self.find_covering_node(ast_start, ast_end) {
            let depth = self.calculate_nesting_depth(node);
            return depth > 2;
        }
        false
    }

    /// Calculate nesting depth of a node
    fn calculate_nesting_depth(&self, target: &CstNode) -> usize {
        self.calculate_depth_recursive(&self.tree.root, target, 0)
    }

    fn calculate_depth_recursive(
        &self,
        current: &CstNode,
        target: &CstNode,
        depth: usize,
    ) -> usize {
        if std::ptr::eq(current, target) {
            return depth;
        }

        let increment = if current.kind == "function_definition"
            || current.kind == "class_definition"
            || current.kind == "decorated_definition"
        {
            1
        } else {
            0
        };

        for child in &current.children {
            if child.start_byte <= target.start_byte && child.end_byte >= target.end_byte {
                let result = self.calculate_depth_recursive(child, target, depth + increment);
                if result > 0 {
                    return result;
                }
            }
        }

        0
    }

    /// Extract decorators for a definition
    #[must_use]
    pub fn get_decorators(&self, ast_start: usize, ast_end: usize) -> Vec<&CstNode> {
        let (expanded_start, _) = self.expand_for_decorators(ast_start, ast_end);
        if expanded_start == ast_start {
            return vec![]; // No decorators
        }

        // Find decorator nodes in the expanded range
        if let Some(decorated) = self.find_covering_node(expanded_start, ast_end) {
            if decorated.kind == "decorated_definition" {
                return decorated.find_by_kind("decorator");
            }
        }

        vec![]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cst::parser::CstParser;

    #[test]
    fn test_expand_for_decorators() {
        let source = r#"@decorator
@another
def foo():
    pass
"#;
        let mut parser = CstParser::new().unwrap();
        let tree = parser.parse(source).unwrap();
        let mapper = AstCstMapper::new(tree);

        // Find 'def foo' start (after decorators)
        let def_start = source.find("def").unwrap();
        let def_end = source.find("pass").unwrap() + 4;

        let (expanded_start, _) = mapper.expand_for_decorators(def_start, def_end);

        // Should include the decorators
        assert!(expanded_start < def_start);
    }

    #[test]
    fn test_precise_range_includes_preceding_comments() {
        let source = r#"# This describes foo
@decorator
def foo():
    pass
"#;
        let mut parser = CstParser::new().unwrap();
        let tree = parser.parse(source).unwrap();
        let mapper = AstCstMapper::new(tree);

        let def_start = source.find("def").unwrap();
        let def_end = source.find("pass").unwrap() + 4;

        let (precise_start, _) = mapper.precise_range_for_def(def_start, def_end);

        // Should include the comment
        assert_eq!(precise_start, 0);
    }

    #[test]
    fn test_interleaved_comments_detection() {
        let source = r#"def foo():
    x = 1
    # This is interleaved
    return x
"#;
        let mut parser = CstParser::new().unwrap();
        let tree = parser.parse(source).unwrap();
        let mapper = AstCstMapper::new(tree);

        let has_interleaved = mapper.has_interleaved_comments(0, source.len());
        assert!(has_interleaved);
    }

    #[test]
    fn test_no_interleaved_comments_for_simple_function() {
        let source = r#"def simple():
    return 42
"#;
        let mut parser = CstParser::new().unwrap();
        let tree = parser.parse(source).unwrap();
        let mapper = AstCstMapper::new(tree);

        let has_interleaved = mapper.has_interleaved_comments(0, source.len());
        assert!(!has_interleaved);
    }
}
