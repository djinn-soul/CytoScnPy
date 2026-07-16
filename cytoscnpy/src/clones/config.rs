//! Configuration for clone detection.

use serde::{Deserialize, Serialize};

/// Clone detection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(clippy::struct_excessive_bools)] // Configuration flags for clone detection features
pub struct CloneConfig {
    /// Minimum similarity threshold (0.0 - 1.0)
    pub min_similarity: f64,

    /// Minimum lines for a code block to be considered
    pub min_lines: usize,

    /// Maximum lines for a code block (performance limit)
    pub max_lines: usize,

    /// LSH number of bands (affects recall vs precision)
    pub lsh_bands: usize,

    /// LSH rows per band
    pub lsh_rows: usize,

    /// Auto-fix confidence threshold (0-100)
    pub auto_fix_threshold: u8,

    /// Suggestion threshold (0-100)
    pub suggest_threshold: u8,

    /// Include test files in detection
    pub include_tests: bool,

    /// Detect Type-1 clones (exact)
    pub detect_type1: bool,

    /// Detect Type-2 clones (renamed)
    pub detect_type2: bool,

    /// Detect Type-3 clones (near-miss)
    pub detect_type3: bool,

    /// Threshold for Type-1 (Exact): both raw and normalized must be >= this (0.0-1.0)
    pub type1_threshold: f64,

    /// Legacy Type-2 raw threshold kept for config compatibility.
    /// Type-2 is classified when normalized similarity is exact but raw is not.
    pub type2_raw_max: f64,

    /// Enable CFG-based behavioral validation for clone pairs.
    /// When enabled, uses control flow graph fingerprinting as a secondary
    /// filter to verify clone pairs have similar behavioral structure.
    /// Only available with the `cfg` feature.
    pub cfg_validation: bool,

    /// Minimum CFG similarity score (0.0-1.0) required to keep a pair when
    /// `cfg_validation` is enabled and both control flow graphs build
    /// successfully.
    pub cfg_similarity_threshold: f64,

    /// LSH bucket size at/above which a bucket is treated as boilerplate
    /// and skipped (avoids O(n^2) blowups on very common patterns).
    /// Raising this trades more comparisons for fewer missed clones in
    /// buckets of widely-duplicated code.
    pub lsh_boilerplate_threshold: usize,

    /// Hard cap on candidate pairs produced by LSH bucketing. Candidates
    /// beyond this cap are silently dropped. Raising this trades memory
    /// and comparison time for fewer missed clones on very large inputs.
    pub lsh_max_candidates: usize,
}

impl Default for CloneConfig {
    fn default() -> Self {
        Self {
            min_similarity: 0.80,
            min_lines: 5,
            max_lines: 500,
            lsh_bands: 20,
            lsh_rows: 5,
            auto_fix_threshold: 90,
            suggest_threshold: 60,
            include_tests: false,
            detect_type1: true,
            detect_type2: true,
            detect_type3: true,
            type1_threshold: 0.95, // Both raw and normalized >= 95% for exact
            type2_raw_max: 0.90,   // Kept for config compatibility
            cfg_validation: false, // Disabled by default (requires `cfg` feature)
            cfg_similarity_threshold: 0.7,
            lsh_boilerplate_threshold: crate::constants::BOILERPLATE_THRESHOLD,
            lsh_max_candidates: 500_000,
        }
    }
}

impl CloneConfig {
    /// Validate configuration invariants before detection starts.
    pub fn validate(&self) -> Result<(), String> {
        for (name, value) in [
            ("min_similarity", self.min_similarity),
            ("type1_threshold", self.type1_threshold),
            ("type2_raw_max", self.type2_raw_max),
            ("cfg_similarity_threshold", self.cfg_similarity_threshold),
        ] {
            if !value.is_finite() || !(0.0..=1.0).contains(&value) {
                return Err(format!("{name} must be a finite value between 0.0 and 1.0"));
            }
        }
        if self.min_lines == 0 || self.min_lines > self.max_lines {
            return Err("min_lines must be positive and no greater than max_lines".to_owned());
        }
        if self.lsh_bands == 0 || self.lsh_rows == 0 {
            return Err("LSH bands and rows must be positive".to_owned());
        }
        if self.lsh_bands.checked_mul(self.lsh_rows).is_none() {
            return Err("LSH signature size is too large".to_owned());
        }
        if self.lsh_boilerplate_threshold < 2 || self.lsh_max_candidates == 0 {
            return Err("LSH limits must allow at least one candidate pair".to_owned());
        }
        if self.auto_fix_threshold > 100 || self.suggest_threshold > 100 {
            return Err("confidence thresholds must be between 0 and 100".to_owned());
        }
        Ok(())
    }

    /// Builder: set minimum similarity
    #[must_use]
    pub const fn with_min_similarity(mut self, threshold: f64) -> Self {
        self.min_similarity = threshold;
        self
    }

    /// Builder: set auto-fix threshold
    #[must_use]
    pub const fn with_auto_fix_threshold(mut self, threshold: u8) -> Self {
        self.auto_fix_threshold = threshold;
        self
    }

    /// Builder: set suggestion threshold
    #[must_use]
    pub const fn with_suggest_threshold(mut self, threshold: u8) -> Self {
        self.suggest_threshold = threshold;
        self
    }

    /// Builder: include test files
    #[must_use]
    pub const fn with_tests(mut self, include: bool) -> Self {
        self.include_tests = include;
        self
    }

    /// Builder: configure which clone types to detect
    #[must_use]
    pub const fn with_clone_types(mut self, type1: bool, type2: bool, type3: bool) -> Self {
        self.detect_type1 = type1;
        self.detect_type2 = type2;
        self.detect_type3 = type3;
        self
    }

    /// Builder: enable CFG-based behavioral validation
    ///
    /// When enabled, uses control flow graph fingerprinting to verify
    /// clone pairs have similar behavioral structure. Requires the `cfg` feature.
    #[must_use]
    pub const fn with_cfg_validation(mut self, enable: bool) -> Self {
        self.cfg_validation = enable;
        self
    }
}
