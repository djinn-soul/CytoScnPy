//! Clone detection module for CytoScnPy.
//!
//! This module provides code clone detection with Type-1/2/3 support:
//! - Type-1: Exact clones (whitespace/comment differences)
//! - Type-2: Renamed identifiers/literals
//! - Type-3: Near-miss clones (statements added/removed)
//!
//! For code rewriting, use the shared `crate::fix` module.

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

use indicatif::ProgressBar;
use std::hash::Hasher;
use std::path::PathBuf;
use std::sync::Arc;

/// Main clone detector orchestrator
pub struct CloneDetector {
    config: CloneConfig,
    /// Progress bar for tracking detection progress (shared with main analyzer)
    pub progress_bar: Option<Arc<ProgressBar>>,
}

impl CloneDetector {
    /// Create a new clone detector with default configuration
    #[must_use]
    pub fn new() -> Self {
        Self {
            config: CloneConfig::default(),
            progress_bar: None,
        }
    }

    /// Create with custom configuration
    #[must_use]
    pub const fn with_config(config: CloneConfig) -> Self {
        Self {
            config,
            progress_bar: None,
        }
    }

    /// Number of files to process per chunk to prevent OOM on large projects.
    /// Detect clones from file paths with chunked processing (OOM-safe).
    ///
    /// This method processes files in chunks to prevent memory exhaustion:
    /// 1. Read files in batches of `crate::constants::CHUNK_SIZE` using rayon
    /// 2. Parse and extract fingerprints, then drop source content immediately
    /// 3. Compare fingerprints (lightweight hashes) to find candidates
    /// 4. For matched pairs, reload specific files to generate precise results
    #[must_use]
    pub fn detect_from_paths(&self, paths: &[PathBuf]) -> CloneDetectionResult {
        // Phase 1: Extract fingerprints from files
        let fingerprints = self.extract_fingerprints(paths);

        // Phase 2: LSH candidate pruning using lightweight signatures
        let hasher = hasher::LshHasher::new(self.config.lsh_bands, self.config.lsh_rows);
        let candidates = hasher.find_candidates_from_fingerprints(&fingerprints);

        // Phase 3, 4 & 5: Precise similarity calculation and grouping
        self.find_and_group_clones(&fingerprints, candidates)
    }

    /// Helper to extract fingerprints from files in chunks
    fn extract_fingerprints(&self, paths: &[PathBuf]) -> Vec<parser::CloneFingerprint> {
        use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};

        // Update progress bar for Phase 1
        if let Some(ref pb) = self.progress_bar {
            pb.set_length(paths.len() as u64);
            pb.set_position(0);
            pb.set_message("Extracting clone fingerprints...");
            pb.set_style(
                indicatif::ProgressStyle::default_bar()
                    .template("{spinner:.cyan} [{bar:40.cyan/blue}] {percent}% - Analyzing file fingerprints...")
                    .unwrap_or_else(|_| indicatif::ProgressStyle::default_bar())
                    .progress_chars("█▓░"),
            );
        }

        let mut fingerprints: Vec<parser::CloneFingerprint> = Vec::new();
        let id_normalizer = Normalizer::for_clone_type(CloneType::Type2);
        let hasher = hasher::LshHasher::new(self.config.lsh_bands, self.config.lsh_rows);

        let min_lines = self.config.min_lines;
        let max_lines = self.config.max_lines;

        for chunk in paths.chunks(crate::constants::CHUNK_SIZE) {
            let chunk_fingerprints: Vec<parser::CloneFingerprint> = chunk
                .par_iter()
                .filter_map(|path| {
                    if crate::CANCELLED.load(std::sync::atomic::Ordering::Relaxed) {
                        return None;
                    }
                    let source = std::fs::read_to_string(path).ok()?;
                    let subtrees = parser::extract_subtrees(&source, path).ok()?;

                    if let Some(ref pb) = self.progress_bar {
                        pb.inc(1);
                    }

                    Some(
                        subtrees
                            .into_iter()
                            .filter_map(|s| {
                                let line_count =
                                    s.end_line.saturating_sub(s.start_line).saturating_add(1);
                                if line_count < min_lines || line_count > max_lines {
                                    return None;
                                }

                                let normalized = id_normalizer.normalize(&s);
                                let lsh_signature = hasher.signature(&normalized);

                                // Structural hash for fast Type-1/2 check
                                let mut struct_hasher = rustc_hash::FxHasher::default();
                                for kind in normalized.kind_sequence() {
                                    use std::hash::Hash;
                                    kind.hash(&mut struct_hasher);
                                }

                                Some(parser::CloneFingerprint {
                                    file: s.file,
                                    start_byte: s.start_byte,
                                    end_byte: s.end_byte,
                                    start_line: s.start_line,
                                    end_line: s.end_line,
                                    name: s.name,
                                    node_type: s.node_type,
                                    lsh_signature,
                                    structural_hash: struct_hasher.finish(),
                                })
                            })
                            .collect::<Vec<_>>(),
                    )
                })
                .flatten()
                .collect();

            fingerprints.extend(chunk_fingerprints);
        }
        fingerprints
    }

    /// Helper to find clone pairs and group them
    fn find_and_group_clones(
        &self,
        fingerprints: &[parser::CloneFingerprint],
        candidates: Vec<(usize, usize)>,
    ) -> CloneDetectionResult {
        // Phase 3 & 4: Precise similarity calculation (Reloading required files)
        let similarity_calc = TreeSimilarity::default();
        let mut pairs = Vec::new();

        // Subtree cache to avoid re-parsing the same files
        let mut subtree_cache: std::collections::HashMap<PathBuf, Vec<parser::Subtree>> =
            std::collections::HashMap::new();
        let total_candidates = candidates.len();

        // Update progress bar if available
        if let Some(ref pb) = self.progress_bar {
            pb.set_length(total_candidates as u64);
            pb.set_position(0);
            pb.set_message(""); // Clear message
            pb.set_style(
                indicatif::ProgressStyle::default_bar()
                    .template(
                        "{spinner:.cyan} [{bar:40.cyan/blue}] {percent}% - Checking code similarity...",
                    )
                    .unwrap_or_else(|_| indicatif::ProgressStyle::default_bar())
                    .progress_chars("█▓░"),
            );
        }

        for (idx, (i, j)) in candidates.into_iter().enumerate() {
            if let Some(ref pb) = self.progress_bar {
                if idx % 100 == 0 || idx == total_candidates - 1 {
                    pb.set_position(idx as u64);
                }
            }

            let fp_a = &fingerprints[i];
            let fp_b = &fingerprints[j];

            // Get or load subtrees for file A
            if !subtree_cache.contains_key(&fp_a.file) {
                if let Ok(source) = std::fs::read_to_string(&fp_a.file) {
                    if let Ok(st) = parser::extract_subtrees(&source, &fp_a.file) {
                        subtree_cache.insert(fp_a.file.clone(), st);
                    }
                }
            }

            // Get or load subtrees for file B
            if !subtree_cache.contains_key(&fp_b.file) {
                if let Ok(source) = std::fs::read_to_string(&fp_b.file) {
                    if let Ok(st) = parser::extract_subtrees(&source, &fp_b.file) {
                        subtree_cache.insert(fp_b.file.clone(), st);
                    }
                }
            }

            let sub_a = subtree_cache
                .get(&fp_a.file)
                .and_then(|st| st.iter().find(|s| s.start_byte == fp_a.start_byte));

            let sub_b = subtree_cache
                .get(&fp_b.file)
                .and_then(|st| st.iter().find(|s| s.start_byte == fp_b.start_byte));

            let (Some(sub_a), Some(sub_b)) = (sub_a, sub_b) else {
                continue;
            };

            let raw_normalizer = Normalizer::for_clone_type(CloneType::Type1);
            let id_normalizer = Normalizer::for_clone_type(CloneType::Type2);

            let raw_a = raw_normalizer.normalize(sub_a);
            let raw_b = raw_normalizer.normalize(sub_b);
            let id_a = id_normalizer.normalize(sub_a);
            let id_b = id_normalizer.normalize(sub_b);

            let raw_sim = similarity_calc.similarity(&raw_a, &raw_b);
            let id_sim = similarity_calc.similarity(&id_a, &id_b);

            if id_sim >= self.config.min_similarity {
                let t1 = self.config.type1_threshold;
                let t2_raw = self.config.type2_raw_max;

                let clone_type = if raw_sim >= t1 && id_sim >= t1 {
                    CloneType::Type1
                } else if id_sim >= t1 && raw_sim < t2_raw {
                    CloneType::Type2
                } else if id_sim >= t1 {
                    CloneType::Type1
                } else {
                    CloneType::Type3
                };

                if !self.is_type_enabled(clone_type) {
                    continue;
                }

                pairs.push(ClonePair {
                    instance_a: fp_a.to_instance(),
                    instance_b: fp_b.to_instance(),
                    similarity: id_sim,
                    clone_type,
                    edit_distance: similarity_calc.edit_distance(&id_a, &id_b),
                });
            }
        }

        if let Some(ref pb) = self.progress_bar {
            pb.finish_and_clear();
        }

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
    #[must_use]
    pub fn detect(&self, files: &[(PathBuf, String)]) -> CloneDetectionResult {
        let mut all_subtrees = Vec::new();
        let min_lines = self.config.min_lines;
        let max_lines = self.config.max_lines;

        for (path, source) in files {
            if let Ok(subtrees) = parser::extract_subtrees(source, path) {
                all_subtrees.extend(subtrees);
            }
        }

        // For backward compatibility, we convert subtrees to fingerprints then run detection
        let id_normalizer = Normalizer::for_clone_type(CloneType::Type2);
        let hasher = hasher::LshHasher::new(self.config.lsh_bands, self.config.lsh_rows);

        let filtered_subtrees: Vec<&parser::Subtree> = all_subtrees
            .iter()
            .filter(|s| {
                let line_count = s.end_line.saturating_sub(s.start_line).saturating_add(1);
                line_count >= min_lines && line_count <= max_lines
            })
            .collect();

        let fingerprints: Vec<_> = filtered_subtrees
            .iter()
            .map(|s| {
                let normalized = id_normalizer.normalize(s);
                let mut struct_hasher = rustc_hash::FxHasher::default();
                for kind in normalized.kind_sequence() {
                    use std::hash::Hash;
                    kind.hash(&mut struct_hasher);
                }
                parser::CloneFingerprint {
                    file: s.file.clone(),
                    start_byte: s.start_byte,
                    end_byte: s.end_byte,
                    start_line: s.start_line,
                    end_line: s.end_line,
                    name: s.name.clone(),
                    node_type: s.node_type,
                    lsh_signature: hasher.signature(&normalized),
                    structural_hash: struct_hasher.finish(),
                }
            })
            .collect();

        // Then we would normally call detect_from_paths, but here we have data in memory.
        // Internal helper could be extracted if needed, but for now we reuse the core logic.
        let candidates = hasher.find_candidates_from_fingerprints(&fingerprints);
        let similarity_calc = TreeSimilarity::default();
        let mut pairs = Vec::new();

        for (i, j) in candidates {
            let id_a = id_normalizer.normalize(filtered_subtrees[i]);
            let id_b = id_normalizer.normalize(filtered_subtrees[j]);
            let id_sim = similarity_calc.similarity(&id_a, &id_b);

            if id_sim >= self.config.min_similarity {
                let clone_type = similarity_calc.classify_by_similarity(id_sim);
                if !self.is_type_enabled(clone_type) {
                    continue;
                }

                pairs.push(ClonePair {
                    instance_a: fingerprints[i].to_instance(),
                    instance_b: fingerprints[j].to_instance(),
                    similarity: id_sim,
                    clone_type,
                    edit_distance: similarity_calc.edit_distance(&id_a, &id_b),
                });
            }
        }

        let groups = self.group_clones(&pairs);
        CloneDetectionResult {
            pairs,
            groups: groups.clone(),
            summary: CloneSummary::from_groups(&groups),
        }
    }

    /// Group related clone pairs into clone groups
    #[allow(clippy::unused_self)]
    fn group_clones(&self, _pairs: &[ClonePair]) -> Vec<CloneGroup> {
        // TODO: implement union-find grouping
        Vec::new()
    }

    fn is_type_enabled(&self, clone_type: CloneType) -> bool {
        match clone_type {
            CloneType::Type1 => self.config.detect_type1,
            CloneType::Type2 => self.config.detect_type2,
            CloneType::Type3 => self.config.detect_type3,
        }
    }

    /// Validate clone pairs using CFG behavioral analysis
    ///
    /// Filters out pairs where the control flow structure differs significantly.
    /// Only applies to function-level clones (functions have meaningful CFG).
    #[cfg(feature = "cfg")]
    #[allow(dead_code)] // Optional validation path, enabled for future CLI wiring.
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
