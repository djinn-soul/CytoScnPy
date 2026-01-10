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
pub use parser::{extract_subtrees, Subtree, SubtreeNode, SubtreeType};
pub use similarity::TreeSimilarity;
pub use types::{
    CloneFinding, CloneGroup, CloneInstance, ClonePair, CloneRelation, CloneSummary, CloneType,
    NodeKind,
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
    pub const fn with_config(config: CloneConfig) -> Self {
        Self {
            confidence_scorer: ConfidenceScorer::new(
                config.auto_fix_threshold,
                config.suggest_threshold,
            ),
            config,
        }
    }

    /// Number of files to process per chunk to prevent OOM on large projects.
    const CHUNK_SIZE: usize = 500;

    /// Detect clones from file paths with chunked processing (OOM-safe).
    ///
    /// This method processes files in chunks to prevent memory exhaustion:
    /// 1. Read files in batches of `CHUNK_SIZE`
    /// 2. Parse and extract fingerprints, then drop source content
    /// 3. Compare fingerprints (lightweight) to find candidates
    /// 4. For matched pairs, reload specific files to generate findings
    #[must_use]
    pub fn detect_from_paths(&self, paths: &[PathBuf]) -> CloneDetectionResult {
        use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};

        // Phase 1: Chunked fingerprint extraction
        // Process files in chunks to limit peak memory usage
        let mut all_subtrees: Vec<parser::Subtree> = Vec::new();

        for chunk in paths.chunks(Self::CHUNK_SIZE) {
            // Read and parse chunk
            let chunk_subtrees: Vec<parser::Subtree> = chunk
                .par_iter()
                .filter_map(|path| {
                    let source = std::fs::read_to_string(path).ok()?;
                    parser::extract_subtrees(&source, path).ok()
                })
                .flatten()
                .collect();

            all_subtrees.extend(chunk_subtrees);
            // Source content dropped here when chunk goes out of scope
        }

        // Phase 2-5: Use existing detection logic on extracted subtrees
        self.detect_from_subtrees(&all_subtrees)
    }

    /// Internal detection from pre-extracted subtrees (shared by detect and `detect_from_paths`)
    fn detect_from_subtrees(&self, all_subtrees: &[parser::Subtree]) -> CloneDetectionResult {
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

        // Phase 4.5: CFG-based behavioral validation (optional)
        // When enabled, filters out clone pairs where the control flow differs significantly
        #[cfg(feature = "cfg")]
        let pairs = if self.config.cfg_validation {
            self.validate_with_cfg(pairs, all_subtrees)
        } else {
            pairs
        };

        // Phase 5: Group clones
        let groups = self.group_clones(&pairs);
        let summary = CloneSummary::from_groups(&groups);

        CloneDetectionResult {
            pairs,
            groups,
            summary,
        }
    }

    /// Detect clones in the given source files (backward compatible API)
    ///
    /// Parse errors are silently skipped (reported in main analysis).
    #[must_use]
    pub fn detect(&self, files: &[(PathBuf, String)]) -> CloneDetectionResult {
        let mut all_subtrees = Vec::new();

        // Phase 1: Parse and extract subtrees (skip files with parse errors)
        for (path, source) in files {
            match parser::extract_subtrees(source, path) {
                Ok(subtrees) => all_subtrees.extend(subtrees),
                Err(_e) => {
                    // Skip unparsable files silently - they'll be reported in the main analysis
                }
            }
        }

        // Delegate to shared detection logic
        self.detect_from_subtrees(&all_subtrees)
    }

    /// Group related clone pairs into clone groups
    #[allow(clippy::unused_self)]
    const fn group_clones(&self, _pairs: &[ClonePair]) -> Vec<CloneGroup> {
        // TODO: implement union-find grouping
        Vec::new()
    }

    /// Validate clone pairs using CFG behavioral analysis
    ///
    /// Filters out pairs where the control flow structure differs significantly.
    /// Only applies to function-level clones (functions have meaningful CFG).
    #[cfg(feature = "cfg")]
    #[allow(clippy::unused_self)]
    fn validate_with_cfg(
        &self,
        pairs: Vec<ClonePair>,
        subtrees: &[parser::Subtree],
    ) -> Vec<ClonePair> {
        use crate::cfg::Cfg;
        use parser::SubtreeType;

        // Build a map from (file, start_byte) to subtree index for lookup
        let subtree_map: std::collections::HashMap<(PathBuf, usize), usize> = subtrees
            .iter()
            .enumerate()
            .map(|(i, s)| ((s.file.clone(), s.start_byte), i))
            .collect();

        pairs
            .into_iter()
            .filter(|pair| {
                // Find the subtrees for both instances
                let key_a = (pair.instance_a.file.clone(), pair.instance_a.start_byte);
                let key_b = (pair.instance_b.file.clone(), pair.instance_b.start_byte);

                let (Some(&idx_a), Some(&idx_b)) =
                    (subtree_map.get(&key_a), subtree_map.get(&key_b))
                else {
                    return true; // Keep pair if subtrees not found
                };

                let subtree_a = &subtrees[idx_a];
                let subtree_b = &subtrees[idx_b];

                // Only validate function-level clones (classes don't have meaningful single CFG)
                let is_function_a = matches!(
                    subtree_a.node_type,
                    SubtreeType::Function | SubtreeType::AsyncFunction | SubtreeType::Method
                );
                let is_function_b = matches!(
                    subtree_b.node_type,
                    SubtreeType::Function | SubtreeType::AsyncFunction | SubtreeType::Method
                );

                if !is_function_a || !is_function_b {
                    return true; // Keep non-function clones (class clones)
                }

                // Build CFGs from source
                let name_a = subtree_a.name.as_deref().unwrap_or("func");
                let name_b = subtree_b.name.as_deref().unwrap_or("func");

                let cfg_a = Cfg::from_source(&subtree_a.source_slice, name_a);
                let cfg_b = Cfg::from_source(&subtree_b.source_slice, name_b);

                match (cfg_a, cfg_b) {
                    (Some(a), Some(b)) => {
                        // Use similarity score with threshold
                        let similarity = a.similarity_score(&b);
                        similarity >= 0.7 // Keep if CFG similarity >= 70%
                    }
                    _ => true, // Keep pair if CFG construction fails
                }
            })
            .collect()
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
