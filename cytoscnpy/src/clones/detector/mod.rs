use super::{CloneConfig, CloneGroup, ClonePair, CloneSummary, CloneType};
use indicatif::ProgressBar;
use std::sync::Arc;

mod cfg_validation;
mod in_memory;
mod paths;

/// Main clone detector orchestrator
pub struct CloneDetector {
    pub(super) config: CloneConfig,
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
    #[must_use]
    pub fn detect_from_paths(&self, paths: &[std::path::PathBuf]) -> CloneDetectionResult {
        paths::detect_from_paths(self, paths)
    }

    /// Detect clones in the given source files (backward compatible API)
    #[must_use]
    pub fn detect(&self, files: &[(std::path::PathBuf, String)]) -> CloneDetectionResult {
        in_memory::detect_from_memory(self, files)
    }

    /// Group related clone pairs into clone groups
    #[allow(clippy::unused_self)]
    pub(super) fn group_clones(&self, _pairs: &[ClonePair]) -> Vec<CloneGroup> {
        // TODO: implement union-find grouping
        Vec::new()
    }

    pub(super) fn is_type_enabled(&self, clone_type: CloneType) -> bool {
        match clone_type {
            CloneType::Type1 => self.config.detect_type1,
            CloneType::Type2 => self.config.detect_type2,
            CloneType::Type3 => self.config.detect_type3,
        }
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
