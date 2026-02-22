use crate::clones::{ClonePair, CloneType, NodeKind};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// A finding for JSON output, representing a detected clone
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloneFinding {
    /// Rule ID for the finding
    pub rule_id: String,
    /// Finding message
    pub message: String,
    /// Severity level
    pub severity: String,
    /// File where the clone was found
    pub file: PathBuf,
    /// Start line of the clone
    pub line: usize,
    /// End line of the clone
    pub end_line: usize,
    /// Start byte offset (from AST parser)
    pub start_byte: usize,
    /// End byte offset (from AST parser)
    pub end_byte: usize,
    /// Clone type (Type1, Type2, Type3)
    pub clone_type: CloneType,
    /// Similarity score (0.0 - 1.0)
    pub similarity: f64,
    /// Name of the function/class (if any)
    pub name: Option<String>,
    /// Related clone location
    pub related_clone: CloneRelation,
    /// Confidence score for auto-fix (0-100)
    pub fix_confidence: u8,
    /// Whether this is the canonical (kept) or duplicate (removable)
    pub is_duplicate: bool,
    /// Refactoring suggestion for this clone
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
    /// Kind of code element (function, class, method) for context
    pub node_kind: NodeKind,
}

/// Relation to another clone (for highlighting in JSON)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloneRelation {
    /// File containing the related clone
    pub file: PathBuf,
    /// Start line of the related clone
    pub line: usize,
    /// End line of the related clone
    pub end_line: usize,
    /// Name of the related function/class
    pub name: Option<String>,
}

impl CloneFinding {
    /// Create a finding from a clone pair
    #[must_use]
    pub fn from_pair(pair: &ClonePair, is_duplicate: bool, fix_confidence: u8) -> Self {
        let (this, other) = if is_duplicate {
            (&pair.instance_b, &pair.instance_a)
        } else {
            (&pair.instance_a, &pair.instance_b)
        };

        let clone_type_str = match pair.clone_type {
            CloneType::Type1 => "exact",
            CloneType::Type2 => "renamed",
            CloneType::Type3 => "near-miss",
        };

        let other_file_name = other
            .file
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("file");
        let other_name = other.name.as_deref().unwrap_or(other_file_name);
        let location_ref = format!("{}:{}", other_file_name, other.start_line);

        let message = if is_duplicate {
            format!(
                "Duplicate of {} at {} ({} clone, {:.0}% similar)",
                other_name,
                location_ref,
                clone_type_str,
                pair.similarity * 100.0
            )
        } else {
            format!(
                "Similar to {} at {} ({} clone, {:.0}% match)",
                other_name,
                location_ref,
                clone_type_str,
                pair.similarity * 100.0
            )
        };

        Self {
            rule_id: format!(
                "CSP-C{}",
                match pair.clone_type {
                    CloneType::Type1 => "100",
                    CloneType::Type2 => "200",
                    CloneType::Type3 => "300",
                }
            ),
            message,
            severity: if is_duplicate {
                "WARNING".to_owned()
            } else {
                "INFO".to_owned()
            },
            file: this.file.clone(),
            line: this.start_line,
            end_line: this.end_line,
            start_byte: this.start_byte,
            end_byte: this.end_byte,
            clone_type: pair.clone_type,
            similarity: pair.similarity,
            name: this.name.clone(),
            related_clone: CloneRelation {
                file: other.file.clone(),
                line: other.start_line,
                end_line: other.end_line,
                name: other.name.clone(),
            },
            fix_confidence,
            is_duplicate,
            suggestion: None,
            node_kind: this.node_kind,
        }
    }
}
