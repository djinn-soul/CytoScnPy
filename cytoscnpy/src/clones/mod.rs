//! Clone detection module for CytoScnPy.
//!
//! This module provides code clone detection with Type-1/2/3 support:
//! - Type-1: Exact clones (whitespace/comment differences)
//! - Type-2: Renamed identifiers/literals
//! - Type-3: Near-miss clones (statements added/removed)
//!
//! For code rewriting, use the shared `crate::fix` module.

// Allow dead code for WIP module - will be used when CLI integration is added
#![allow(dead_code)]

mod confidence;
mod config;
mod hasher;
mod normalizer;
mod parser;
mod similarity;
mod types;

// Re-exports
pub use confidence::{ConfidenceScorer, FixConfidence, FixContext, FixDecision};
pub use config::CloneConfig;
pub use normalizer::Normalizer;
pub use parser::AstParser;
pub use similarity::TreeSimilarity;
pub use types::{
    CloneFinding, CloneGroup, CloneInstance, ClonePair, CloneRelation, CloneSummary, CloneType,
};

// Re-export from shared fix module for convenience
pub use crate::fix::ByteRangeRewriter;

use std::path::PathBuf;

/// Main clone detector orchestrator
pub struct CloneDetector {
    config: CloneConfig,
    confidence_scorer: ConfidenceScorer,
}

impl CloneDetector {
    /// Create a new clone detector with default configuration
    #[must_use]
    pub fn new() -> Self {
        Self {
            config: CloneConfig::default(),
            confidence_scorer: ConfidenceScorer::default(),
        }
    }

    /// Create with custom configuration
    #[must_use]
    pub fn with_config(config: CloneConfig) -> Self {
        Self {
            confidence_scorer: ConfidenceScorer::new(
                config.auto_fix_threshold,
                config.suggest_threshold,
            ),
            config,
        }
    }

    /// Detect clones in the given source files
    ///
    /// # Errors
    /// Returns error if parsing fails for any file
    pub fn detect(&self, files: &[(PathBuf, String)]) -> Result<CloneDetectionResult, CloneError> {
        let mut all_subtrees = Vec::new();

        // Phase 1: Parse and extract subtrees
        for (path, source) in files {
            let subtrees = parser::extract_subtrees(source, path)?;
            all_subtrees.extend(subtrees);
        }

        // Phase 2: Create normalizers for both raw and renamed comparison
        let raw_normalizer = Normalizer::for_clone_type(CloneType::Type1); // Preserves identifiers
        let renamed_normalizer = Normalizer::for_clone_type(CloneType::Type2); // Renames identifiers

        let raw_normalized: Vec<_> = all_subtrees
            .iter()
            .map(|s| raw_normalizer.normalize(s))
            .collect();
        let id_normalized: Vec<_> = all_subtrees
            .iter()
            .map(|s| renamed_normalizer.normalize(s))
            .collect();

        // Phase 3: LSH candidate pruning (use identifier-normalized for broad matching)
        let hasher = hasher::LshHasher::new(self.config.lsh_bands, self.config.lsh_rows);
        let candidates = hasher.find_candidates(&id_normalized);

        // Phase 4: Precise similarity calculation with type classification
        let similarity_calc = TreeSimilarity::default();
        let mut pairs = Vec::new();

        for (i, j) in candidates {
            // Calculate both raw and normalized similarity
            let raw_sim = similarity_calc.similarity(&raw_normalized[i], &raw_normalized[j]);
            let id_sim = similarity_calc.similarity(&id_normalized[i], &id_normalized[j]);

            if id_sim >= self.config.min_similarity {
                // Classify clone type using both raw and normalized similarity:
                // Type-1 (Exact Copy): Both raw and normalized are very high
                // Type-2 (Renamed Copy): Normalized high but raw is lower
                // Type-3 (Similar Code): Normalized similarity is moderate
                let t1 = self.config.type1_threshold;
                let t2_raw = self.config.type2_raw_max;

                let clone_type = if raw_sim >= t1 && id_sim >= t1 {
                    CloneType::Type1 // Exact: even raw identifiers match
                } else if id_sim >= t1 && raw_sim < t2_raw {
                    CloneType::Type2 // Renamed: structure same but identifiers differ
                } else if id_sim >= t1 {
                    CloneType::Type1 // Near-exact: small identifier variations
                } else {
                    CloneType::Type3 // Similar: structural differences
                };

                pairs.push(ClonePair {
                    instance_a: all_subtrees[i].to_instance(),
                    instance_b: all_subtrees[j].to_instance(),
                    similarity: id_sim,
                    clone_type,
                    edit_distance: similarity_calc
                        .edit_distance(&id_normalized[i], &id_normalized[j]),
                });
            }
        }

        // Phase 5: Group clones
        let groups = self.group_clones(&pairs);
        let summary = CloneSummary::from_groups(&groups);

        Ok(CloneDetectionResult {
            pairs,
            groups,
            summary,
        })
    }

    /// Group related clone pairs into clone groups
    #[allow(clippy::unused_self)]
    fn group_clones(&self, _pairs: &[ClonePair]) -> Vec<CloneGroup> {
        // TODO: implement union-find grouping
        Vec::new()
    }
}

impl Default for CloneDetector {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of clone detection
#[derive(Debug, Clone)]
pub struct CloneDetectionResult {
    /// All detected clone pairs
    pub pairs: Vec<ClonePair>,
    /// Grouped clones
    pub groups: Vec<CloneGroup>,
    /// Summary statistics
    pub summary: CloneSummary,
}

/// Clone detection error
#[derive(Debug)]
pub enum CloneError {
    /// Error during parsing
    ParseError(String),
    /// IO error
    IoError(std::io::Error),
}

impl std::fmt::Display for CloneError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ParseError(msg) => write!(f, "Parse error: {msg}"),
            Self::IoError(e) => write!(f, "IO error: {e}"),
        }
    }
}

impl std::error::Error for CloneError {}
