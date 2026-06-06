use super::{CloneConfig, CloneGroup, ClonePair, CloneSummary, CloneType};
use indicatif::ProgressBar;
use std::sync::Arc;

mod cfg_validation;
mod grouping;
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
    pub(super) fn group_clones(pairs: &[ClonePair]) -> Vec<CloneGroup> {
        grouping::group_clones(pairs)
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

#[cfg(test)]
mod tests {
    use super::CloneDetector;
    use crate::clones::{CloneInstance, ClonePair, CloneType, NodeKind};
    use std::path::PathBuf;

    fn inst(file: &str, start: usize) -> CloneInstance {
        CloneInstance {
            file: PathBuf::from(file),
            start_line: start,
            end_line: start + 4,
            start_byte: start * 10,
            end_byte: start * 10 + 20,
            normalized_hash: start as u64,
            name: Some(format!("f{start}")),
            node_kind: NodeKind::Function,
        }
    }

    fn pair(
        a: CloneInstance,
        b: CloneInstance,
        similarity: f64,
        clone_type: CloneType,
    ) -> ClonePair {
        ClonePair {
            instance_a: a,
            instance_b: b,
            similarity,
            clone_type,
            edit_distance: 0,
        }
    }

    #[test]
    fn group_clones_empty_input_returns_no_groups() {
        let groups = CloneDetector::group_clones(&[]);
        assert!(groups.is_empty());
    }

    #[test]
    fn group_clones_builds_connected_component_cluster() {
        let first = inst("a.py", 1);
        let second = inst("b.py", 2);
        let third = inst("c.py", 3);
        let pairs = vec![
            pair(first, second.clone(), 0.95, CloneType::Type1),
            pair(second, third, 0.90, CloneType::Type2),
        ];

        let groups = CloneDetector::group_clones(&pairs);
        assert_eq!(groups.len(), 1);

        let group = &groups[0];
        assert_eq!(group.id, 1);
        assert_eq!(group.instances.len(), 3);
        assert_eq!(group.canonical_index, Some(0));
        assert!(group
            .instances
            .iter()
            .any(|instance| instance.file.as_path() == std::path::Path::new("a.py")));
        assert!(group
            .instances
            .iter()
            .any(|instance| instance.file.as_path() == std::path::Path::new("b.py")));
        assert!(group
            .instances
            .iter()
            .any(|instance| instance.file.as_path() == std::path::Path::new("c.py")));
        assert!((group.avg_similarity - 0.925).abs() < 1e-9);
    }

    #[test]
    fn group_clones_keeps_disconnected_components_separate() {
        let first = inst("a.py", 1);
        let second = inst("b.py", 2);
        let third = inst("c.py", 3);
        let fourth = inst("d.py", 4);

        let pairs = vec![
            pair(first, second, 0.91, CloneType::Type2),
            pair(third, fourth, 0.92, CloneType::Type3),
        ];

        let groups = CloneDetector::group_clones(&pairs);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].id, 1);
        assert_eq!(groups[1].id, 2);
        assert_eq!(groups[0].instances.len(), 2);
        assert_eq!(groups[1].instances.len(), 2);
    }
}
