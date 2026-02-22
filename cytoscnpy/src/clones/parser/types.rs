use crate::clones::types::{CloneInstance, NodeKind};
use crate::clones::CloneError;
use ruff_python_ast as ast;
use ruff_python_parser::parse_module;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// A subtree extracted from source code for clone analysis
#[derive(Debug, Clone)]
pub struct Subtree {
    /// Type of node (function, class, etc.)
    pub node_type: SubtreeType,
    /// Name of the function/class (if any)
    pub name: Option<String>,
    /// Start byte offset
    pub start_byte: usize,
    /// End byte offset
    pub end_byte: usize,
    /// Start line (1-indexed)
    pub start_line: usize,
    /// End line (1-indexed)
    pub end_line: usize,
    /// Source file path
    pub file: PathBuf,
    /// Raw source slice
    pub source_slice: String,
    /// Child nodes for tree comparison
    pub children: Vec<SubtreeNode>,
}

/// Type of subtree node
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SubtreeType {
    /// Regular function definition
    Function,
    /// Async function definition
    AsyncFunction,
    /// Class definition
    Class,
    /// Method within a class
    Method,
}

/// A lightweight fingerprint of a code block for the first pass of detection.
/// Stores only metadata and hashes to prevent memory exhaustion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloneFingerprint {
    pub file: PathBuf,
    pub start_byte: usize,
    pub end_byte: usize,
    pub start_line: usize,
    pub end_line: usize,
    pub name: Option<String>,
    pub node_type: SubtreeType,
    /// `MinHash` signature for LSH candidate pruning
    pub lsh_signature: Vec<u64>,
    /// Hash of the normalized structure for Type-1/2 comparison
    pub structural_hash: u64,
}

impl CloneFingerprint {
    /// Convert to a `CloneInstance` (used when generating results)
    #[must_use]
    pub fn to_instance(&self) -> CloneInstance {
        let node_kind = match self.node_type {
            SubtreeType::Function => NodeKind::Function,
            SubtreeType::AsyncFunction => NodeKind::AsyncFunction,
            SubtreeType::Class => NodeKind::Class,
            SubtreeType::Method => NodeKind::Method,
        };

        CloneInstance {
            file: self.file.clone(),
            start_line: self.start_line,
            end_line: self.end_line,
            start_byte: self.start_byte,
            end_byte: self.end_byte,
            normalized_hash: self.structural_hash,
            name: self.name.clone(),
            node_kind,
        }
    }
}

/// A node in the subtree (for edit distance calculation)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SubtreeNode {
    /// Node kind (e.g., "if", "for", "assign", "call")
    pub kind: String,
    /// Optional label (normalized identifier)
    pub label: Option<String>,
    /// Child nodes
    pub children: Vec<SubtreeNode>,
}

impl SubtreeNode {
    /// Count total nodes in this subtree
    #[must_use]
    pub fn size(&self) -> usize {
        1 + self.children.iter().map(SubtreeNode::size).sum::<usize>()
    }
}

impl Subtree {
    /// Convert to a `CloneInstance`
    #[must_use]
    pub fn to_instance(&self) -> CloneInstance {
        use std::hash::{Hash, Hasher};

        let mut hasher = rustc_hash::FxHasher::default();
        for child in &self.children {
            child.kind.hash(&mut hasher);
        }

        let node_kind = match self.node_type {
            SubtreeType::Function => NodeKind::Function,
            SubtreeType::AsyncFunction => NodeKind::AsyncFunction,
            SubtreeType::Class => NodeKind::Class,
            SubtreeType::Method => NodeKind::Method,
        };

        CloneInstance {
            file: self.file.clone(),
            start_line: self.start_line,
            end_line: self.end_line,
            start_byte: self.start_byte,
            end_byte: self.end_byte,
            normalized_hash: hasher.finish(),
            name: self.name.clone(),
            node_kind,
        }
    }
}

/// AST parser for clone detection
pub struct AstParser;

impl AstParser {
    /// Parse source code and return the module
    ///
    /// # Errors
    /// Returns error if parsing fails
    pub fn parse(source: &str) -> Result<ast::ModModule, CloneError> {
        parse_module(source)
            .map(ruff_python_parser::Parsed::into_syntax)
            .map_err(|e| CloneError::ParseError(e.to_string()))
    }
}
