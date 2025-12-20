//! Tree-sitter based CST parser for Python source code.
//!
//! Provides precise byte-range information for safe code rewriting.

use tree_sitter::{Node, Parser};

/// A point in source code (row, column)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Point {
    /// Zero-indexed row number
    pub row: usize,
    /// Zero-indexed column (byte offset within line)
    pub column: usize,
}

impl From<tree_sitter::Point> for Point {
    fn from(p: tree_sitter::Point) -> Self {
        Self {
            row: p.row,
            column: p.column,
        }
    }
}

/// A CST node with exact source location
#[derive(Debug, Clone)]
pub struct CstNode {
    /// Node kind (e.g., "function_definition", "identifier")
    pub kind: String,
    /// Start byte offset (inclusive)
    pub start_byte: usize,
    /// End byte offset (exclusive)
    pub end_byte: usize,
    /// Start point (row, column)
    pub start_point: Point,
    /// End point (row, column)
    pub end_point: Point,
    /// Whether this is a named node (vs anonymous like punctuation)
    pub is_named: bool,
    /// Child nodes
    pub children: Vec<CstNode>,
}

impl CstNode {
    /// Create a `CstNode` from a tree-sitter `Node`
    fn from_ts_node(node: Node<'_>) -> Self {
        let children = (0..node.child_count())
            .filter_map(|i| node.child(i))
            .map(Self::from_ts_node)
            .collect();

        Self {
            kind: node.kind().to_string(),
            start_byte: node.start_byte(),
            end_byte: node.end_byte(),
            start_point: node.start_position().into(),
            end_point: node.end_position().into(),
            is_named: node.is_named(),
            children,
        }
    }

    /// Check if this node's range contains the given byte offset
    #[must_use]
    pub fn contains_byte(&self, byte: usize) -> bool {
        byte >= self.start_byte && byte < self.end_byte
    }

    /// Check if this node's range overlaps with the given range
    #[must_use]
    pub fn overlaps(&self, start: usize, end: usize) -> bool {
        self.start_byte < end && self.end_byte > start
    }

    /// Find the smallest node containing the given byte range
    #[must_use]
    pub fn find_smallest_covering(&self, start: usize, end: usize) -> Option<&CstNode> {
        if !self.overlaps(start, end) {
            return None;
        }

        // Try to find a smaller child that covers the range
        for child in &self.children {
            if child.start_byte <= start && child.end_byte >= end {
                if let Some(smaller) = child.find_smallest_covering(start, end) {
                    return Some(smaller);
                }
                return Some(child);
            }
        }

        // This node is the smallest covering node
        Some(self)
    }

    /// Find all nodes of a specific kind
    #[must_use]
    pub fn find_by_kind(&self, kind: &str) -> Vec<&CstNode> {
        let mut result = Vec::new();
        self.find_by_kind_recursive(kind, &mut result);
        result
    }

    fn find_by_kind_recursive<'a>(&'a self, kind: &str, result: &mut Vec<&'a CstNode>) {
        if self.kind == kind {
            result.push(self);
        }
        for child in &self.children {
            child.find_by_kind_recursive(kind, result);
        }
    }
}

/// A parsed CST tree
#[derive(Debug)]
pub struct CstTree {
    /// Root node of the CST
    pub root: CstNode,
    /// Original source code
    pub source: String,
}

impl CstTree {
    /// Extract a slice of source code by byte range
    #[must_use]
    pub fn slice(&self, start: usize, end: usize) -> &str {
        &self.source[start..end]
    }

    /// Find all function definitions
    #[must_use]
    pub fn find_functions(&self) -> Vec<&CstNode> {
        self.root.find_by_kind("function_definition")
    }

    /// Find all class definitions
    #[must_use]
    pub fn find_classes(&self) -> Vec<&CstNode> {
        self.root.find_by_kind("class_definition")
    }
}

/// Error during CST parsing
#[derive(Debug)]
pub enum CstError {
    /// Failed to create parser
    ParserCreation(String),
    /// Failed to parse source
    ParseFailed,
    /// Invalid UTF-8 in source
    InvalidUtf8,
}

impl std::fmt::Display for CstError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ParserCreation(msg) => write!(f, "Failed to create CST parser: {msg}"),
            Self::ParseFailed => write!(f, "Failed to parse source as Python"),
            Self::InvalidUtf8 => write!(f, "Source contains invalid UTF-8"),
        }
    }
}

impl std::error::Error for CstError {}

/// Tree-sitter based CST parser
pub struct CstParser {
    parser: Parser,
}

impl CstParser {
    /// Create a new CST parser for Python
    ///
    /// # Errors
    /// Returns error if parser creation fails
    pub fn new() -> Result<Self, CstError> {
        let mut parser = Parser::new();

        // Use the LANGUAGE constant exported by tree-sitter-python crate
        parser
            .set_language(&tree_sitter_python::LANGUAGE.into())
            .map_err(|e| CstError::ParserCreation(e.to_string()))?;

        Ok(Self { parser })
    }

    /// Parse source code into a CST
    ///
    /// # Errors
    /// Returns error if parsing fails
    pub fn parse(&mut self, source: &str) -> Result<CstTree, CstError> {
        let tree = self
            .parser
            .parse(source, None)
            .ok_or(CstError::ParseFailed)?;

        let root = CstNode::from_ts_node(tree.root_node());

        Ok(CstTree {
            root,
            source: source.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_function() {
        let source = r#"def foo():
    pass
"#;
        let mut parser = CstParser::new().unwrap();
        let tree = parser.parse(source).unwrap();

        assert_eq!(tree.root.kind, "module");
        let functions = tree.find_functions();
        assert_eq!(functions.len(), 1);
        assert_eq!(functions[0].kind, "function_definition");
    }

    #[test]
    fn test_byte_ranges_accurate() {
        let source = "x = 1";
        let mut parser = CstParser::new().unwrap();
        let tree = parser.parse(source).unwrap();

        assert_eq!(tree.root.start_byte, 0);
        assert_eq!(tree.root.end_byte, source.len());
    }

    #[test]
    fn test_find_smallest_covering() {
        let source = r#"def foo():
    x = 1
    return x
"#;
        let mut parser = CstParser::new().unwrap();
        let tree = parser.parse(source).unwrap();

        // Find node covering "x = 1" (bytes ~15-20 approx)
        let node = tree.root.find_smallest_covering(15, 20);
        assert!(node.is_some());
    }
}
