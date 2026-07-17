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
    pub fn with_config(config: CloneConfig) -> Self {
        assert!(
            config.validate().is_ok(),
            "invalid clone detection configuration"
        );
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

    pub(super) fn should_process_path(&self, path: &std::path::Path) -> bool {
        self.config.include_tests || !crate::utils::is_test_path(&path.to_string_lossy())
    }

    pub(super) fn classify_clone(
        &self,
        raw_similarity: f64,
        normalized_similarity: f64,
    ) -> CloneType {
        let exact = self.config.type1_threshold;
        if raw_similarity >= exact && normalized_similarity >= exact {
            CloneType::Type1
        } else if normalized_similarity >= exact {
            CloneType::Type2
        } else {
            CloneType::Type3
        }
    }

    pub(crate) const fn confidence_thresholds(&self) -> (u8, u8) {
        (
            self.config.auto_fix_threshold,
            self.config.suggest_threshold,
        )
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
    use crate::clones::{CloneConfig, CloneInstance, ClonePair, CloneType, NodeKind};
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

    #[test]
    fn group_clones_splits_sparse_star_component_instead_of_chaining() {
        // Hub `b` is similar to a, c, d, e, but a/c/d/e are NOT similar to
        // each other. Union-find alone would merge all 5 into one group,
        // falsely reporting a~c, a~d, c~e, etc. as clones. Density-based
        // splitting should refuse to report the full 5-way chain.
        let first = inst("a.py", 1);
        let hub = inst("b.py", 2);
        let third = inst("c.py", 3);
        let fourth = inst("d.py", 4);
        let fifth = inst("e.py", 5);

        let pairs = vec![
            pair(hub.clone(), first, 0.91, CloneType::Type2),
            pair(hub.clone(), third, 0.91, CloneType::Type2),
            pair(hub.clone(), fourth, 0.91, CloneType::Type2),
            pair(hub, fifth, 0.91, CloneType::Type2),
        ];

        let groups = CloneDetector::group_clones(&pairs);

        // No group should contain the full sparse 5-member chain.
        assert!(groups.iter().all(|g| g.instances.len() < 5));
        assert_eq!(groups.len(), pairs.len());
        assert!(groups.iter().all(|group| group.instances.len() == 2));
    }

    #[test]
    fn classify_clone_requires_raw_exact_match_for_type1() {
        let detector = CloneDetector::new();

        assert_eq!(detector.classify_clone(0.96, 0.96), CloneType::Type1);
        assert_eq!(detector.classify_clone(0.94, 0.96), CloneType::Type2);
        assert_eq!(detector.classify_clone(0.91, 0.96), CloneType::Type2);
        assert_eq!(detector.classify_clone(0.96, 0.94), CloneType::Type3);
    }

    #[test]
    fn semantic_operator_changes_are_not_exact_type2_clones() {
        let files = vec![
            (
                PathBuf::from("add.py"),
                "def calculate(value):\n    first = value + 1\n    second = first + 2\n    third = second + 3\n    return third\n".to_owned(),
            ),
            (
                PathBuf::from("subtract.py"),
                "def calculate(value):\n    first = value - 1\n    second = first - 2\n    third = second - 3\n    return third\n".to_owned(),
            ),
        ];
        let config = CloneConfig::default()
            .with_min_similarity(1.0)
            .with_tests(true);
        assert!(CloneDetector::with_config(config)
            .detect(&files)
            .pairs
            .is_empty());
    }

    #[test]
    fn renamed_comprehension_and_fstring_clone_is_recalled() {
        let files = vec![
            (
                PathBuf::from("left.py"),
                "def render(values):\n    selected = [value * 2 for value in values if value > 0]\n    total = sum(selected)\n    message = f\"total={total}\"\n    return message\n".to_owned(),
            ),
            (
                PathBuf::from("right.py"),
                "def display(items):\n    chosen = [item * 2 for item in items if item > 0]\n    amount = sum(chosen)\n    text = f\"total={amount}\"\n    return text\n".to_owned(),
            ),
        ];
        let config = CloneConfig::default()
            .with_min_similarity(1.0)
            .with_tests(true);
        let result = CloneDetector::with_config(config).detect(&files);
        assert_eq!(result.pairs.len(), 1);
        assert_eq!(result.pairs[0].clone_type, CloneType::Type2);
    }
}
