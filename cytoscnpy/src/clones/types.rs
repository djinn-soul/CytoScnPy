//! Core types for clone detection.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Clone type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CloneType {
    /// Exact copy (only whitespace/comments differ)
    Type1,
    /// Renamed identifiers/literals
    Type2,
    /// Near-miss (statements added/removed/reordered)
    Type3,
}

impl CloneType {
    /// Get user-friendly display name
    #[must_use]
    pub const fn display_name(&self) -> &'static str {
        match self {
            Self::Type1 => "Exact Copy",
            Self::Type2 => "Renamed Copy",
            Self::Type3 => "Similar Code",
        }
    }

    /// Get short display name for tables
    #[must_use]
    pub const fn short_name(&self) -> &'static str {
        match self {
            Self::Type1 => "Exact",
            Self::Type2 => "Renamed",
            Self::Type3 => "Similar",
        }
    }

    /// Get confidence bonus for this clone type
    #[must_use]
    pub const fn confidence_bonus(&self) -> i8 {
        match self {
            Self::Type1 => 25,
            Self::Type2 => 15,
            Self::Type3 => -10,
        }
    }
}

/// A single clone instance with source location
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloneInstance {
    /// Source file path
    pub file: PathBuf,
    /// Start line (1-indexed)
    pub start_line: usize,
    /// End line (1-indexed, inclusive)
    pub end_line: usize,
    /// Start byte offset
    pub start_byte: usize,
    /// End byte offset
    pub end_byte: usize,
    /// Hash of normalized content
    pub normalized_hash: u64,
    /// Optional function/class name
    pub name: Option<String>,
}

/// A pair of similar code fragments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClonePair {
    /// First clone instance
    pub instance_a: CloneInstance,
    /// Second clone instance
    pub instance_b: CloneInstance,
    /// Similarity score (0.0 - 1.0)
    pub similarity: f64,
    /// Clone type classification
    pub clone_type: CloneType,
    /// Tree edit distance
    pub edit_distance: usize,
}

impl ClonePair {
    /// Check if both instances are in the same file
    #[must_use]
    pub fn is_same_file(&self) -> bool {
        self.instance_a.file == self.instance_b.file
    }

    /// Get the smaller instance (canonical choice)
    #[must_use]
    pub fn canonical(&self) -> &CloneInstance {
        if self.instance_a.start_byte <= self.instance_b.start_byte {
            &self.instance_a
        } else {
            &self.instance_b
        }
    }
}

/// A group of clones (all similar to each other)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloneGroup {
    /// Unique group ID
    pub id: usize,
    /// All instances in this group
    pub instances: Vec<CloneInstance>,
    /// Index of canonical (best) instance
    pub canonical_index: Option<usize>,
    /// Clone type for this group
    pub clone_type: CloneType,
    /// Average similarity within group
    pub avg_similarity: f64,
}

impl CloneGroup {
    /// Get the canonical instance for this group
    #[must_use]
    pub fn canonical(&self) -> Option<&CloneInstance> {
        self.canonical_index.map(|i| &self.instances[i])
    }

    /// Get non-canonical instances (duplicates to potentially remove)
    #[must_use]
    pub fn duplicates(&self) -> Vec<&CloneInstance> {
        self.instances
            .iter()
            .enumerate()
            .filter(|(i, _)| Some(*i) != self.canonical_index)
            .map(|(_, inst)| inst)
            .collect()
    }
}

/// Summary statistics for clone detection
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CloneSummary {
    /// Total number of clone groups
    pub total_groups: usize,
    /// Total number of clone instances
    pub total_instances: usize,
    /// Type-1 clone count
    pub type1_count: usize,
    /// Type-2 clone count
    pub type2_count: usize,
    /// Type-3 clone count
    pub type3_count: usize,
    /// Number of files with clones
    pub files_with_clones: usize,
    /// Average clone size in lines
    pub avg_clone_size_lines: f64,
}

impl CloneSummary {
    /// Create summary from clone groups
    #[must_use]
    pub fn from_groups(groups: &[CloneGroup]) -> Self {
        use rustc_hash::FxHashSet;

        let mut files: FxHashSet<&PathBuf> = FxHashSet::default();
        let mut total_lines = 0usize;
        let mut total_instances = 0usize;
        let mut type1 = 0;
        let mut type2 = 0;
        let mut type3 = 0;

        for group in groups {
            match group.clone_type {
                CloneType::Type1 => type1 += 1,
                CloneType::Type2 => type2 += 1,
                CloneType::Type3 => type3 += 1,
            }

            for instance in &group.instances {
                files.insert(&instance.file);
                total_lines += instance.end_line.saturating_sub(instance.start_line) + 1;
                total_instances += 1;
            }
        }

        Self {
            total_groups: groups.len(),
            total_instances,
            type1_count: type1,
            type2_count: type2,
            type3_count: type3,
            files_with_clones: files.len(),
            avg_clone_size_lines: if total_instances > 0 {
                total_lines as f64 / total_instances as f64
            } else {
                0.0
            },
        }
    }
}

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

        let message = if is_duplicate {
            format!(
                "Duplicate code ({} clone, {:.0}% similar) - consider removing in favor of {}",
                clone_type_str,
                pair.similarity * 100.0,
                other.name.as_deref().unwrap_or("canonical version")
            )
        } else {
            format!(
                "Clone detected ({} clone, {:.0}% similar to {})",
                clone_type_str,
                pair.similarity * 100.0,
                other.name.as_deref().unwrap_or("another function")
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
        }
    }
}
