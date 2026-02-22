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
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::Type1 => "Exact Copy",
            Self::Type2 => "Renamed Copy",
            Self::Type3 => "Similar Code",
        }
    }

    /// Get short display name for tables
    #[must_use]
    pub const fn short_name(self) -> &'static str {
        match self {
            Self::Type1 => "Exact",
            Self::Type2 => "Renamed",
            Self::Type3 => "Similar",
        }
    }

    /// Get confidence bonus for this clone type
    #[must_use]
    pub const fn confidence_bonus(self) -> i8 {
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
    /// Type of code element (function, class, method)
    pub node_kind: NodeKind,
}

/// Kind of code element for context-aware suggestions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeKind {
    /// Regular function
    Function,
    /// Async function
    AsyncFunction,
    /// Class definition
    Class,
    /// Method inside a class
    Method,
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
    pub const fn canonical(&self) -> &CloneInstance {
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
    #[allow(clippy::cast_precision_loss)] // Precision loss is acceptable for summary statistics
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
