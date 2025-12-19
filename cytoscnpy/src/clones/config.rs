//! Configuration for clone detection.

use serde::{Deserialize, Serialize};

/// Clone detection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
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

    /// Threshold for Type-2 (Renamed): raw similarity must be < this (0.0-1.0)
    /// If normalized >= type1_threshold but raw < type2_raw_max, it's Type-2
    pub type2_raw_max: f64,
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
            type2_raw_max: 0.90,   // Raw < 90% indicates renamed identifiers
        }
    }
}

impl CloneConfig {
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
}
