use super::{CloneConfig, CloneGroup, ClonePair, CloneSummary, CloneType};
use indicatif::ProgressBar;
use rustc_hash::FxHashMap;
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
    pub(super) fn group_clones(pairs: &[ClonePair]) -> Vec<CloneGroup> {
        if pairs.is_empty() {
            return Vec::new();
        }

        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        struct InstanceKey<'a> {
            file: &'a std::path::Path,
            start_byte: usize,
            end_byte: usize,
            start_line: usize,
            end_line: usize,
        }

        #[derive(Debug)]
        struct UnionFind {
            parent: Vec<usize>,
            rank: Vec<u8>,
        }

        impl UnionFind {
            fn new(size: usize) -> Self {
                Self {
                    parent: (0..size).collect(),
                    rank: vec![0; size],
                }
            }

            fn find(&mut self, x: usize) -> usize {
                if self.parent[x] != x {
                    let root = self.find(self.parent[x]);
                    self.parent[x] = root;
                }
                self.parent[x]
            }

            fn union(&mut self, a: usize, b: usize) {
                let root_a = self.find(a);
                let root_b = self.find(b);
                if root_a == root_b {
                    return;
                }

                match self.rank[root_a].cmp(&self.rank[root_b]) {
                    std::cmp::Ordering::Less => self.parent[root_a] = root_b,
                    std::cmp::Ordering::Greater => self.parent[root_b] = root_a,
                    std::cmp::Ordering::Equal => {
                        self.parent[root_b] = root_a;
                        self.rank[root_a] = self.rank[root_a].saturating_add(1);
                    }
                }
            }
        }

        let mut index_by_key = FxHashMap::default();
        let mut instances = Vec::new();
        let mut edges = Vec::with_capacity(pairs.len());

        for pair in pairs {
            let key_a = InstanceKey {
                file: pair.instance_a.file.as_path(),
                start_byte: pair.instance_a.start_byte,
                end_byte: pair.instance_a.end_byte,
                start_line: pair.instance_a.start_line,
                end_line: pair.instance_a.end_line,
            };
            let idx_a = *index_by_key.entry(key_a).or_insert_with(|| {
                let idx = instances.len();
                instances.push(pair.instance_a.clone());
                idx
            });

            let key_b = InstanceKey {
                file: pair.instance_b.file.as_path(),
                start_byte: pair.instance_b.start_byte,
                end_byte: pair.instance_b.end_byte,
                start_line: pair.instance_b.start_line,
                end_line: pair.instance_b.end_line,
            };
            let idx_b = *index_by_key.entry(key_b).or_insert_with(|| {
                let idx = instances.len();
                instances.push(pair.instance_b.clone());
                idx
            });

            edges.push((idx_a, idx_b, pair.clone_type, pair.similarity));
        }

        let mut union_find = UnionFind::new(instances.len());
        for (idx_a, idx_b, _, _) in &edges {
            union_find.union(*idx_a, *idx_b);
        }

        let mut members_by_root: FxHashMap<usize, Vec<usize>> = FxHashMap::default();
        for idx in 0..instances.len() {
            let root = union_find.find(idx);
            members_by_root.entry(root).or_default().push(idx);
        }

        let mut groups = Vec::with_capacity(members_by_root.len());
        let mut roots: Vec<usize> = members_by_root.keys().copied().collect();
        roots.sort_unstable();

        for root in roots {
            let mut member_indices = members_by_root.remove(&root).unwrap_or_default();
            if member_indices.len() < 2 {
                continue;
            }

            member_indices.sort_unstable_by(|a, b| {
                let left = &instances[*a];
                let right = &instances[*b];
                left.file
                    .cmp(&right.file)
                    .then_with(|| left.start_byte.cmp(&right.start_byte))
                    .then_with(|| left.end_byte.cmp(&right.end_byte))
                    .then_with(|| left.start_line.cmp(&right.start_line))
                    .then_with(|| left.end_line.cmp(&right.end_line))
            });

            let member_set: std::collections::HashSet<usize> =
                member_indices.iter().copied().collect();

            let mut similarity_total = 0.0;
            let mut similarity_count = 0usize;
            let mut type_counts = [0usize; 3];

            for (idx_a, idx_b, clone_type, similarity) in &edges {
                if member_set.contains(idx_a) && member_set.contains(idx_b) {
                    similarity_total += similarity;
                    similarity_count += 1;
                    match clone_type {
                        CloneType::Type1 => type_counts[0] += 1,
                        CloneType::Type2 => type_counts[1] += 1,
                        CloneType::Type3 => type_counts[2] += 1,
                    }
                }
            }

            let clone_type = if type_counts[0] >= type_counts[1] && type_counts[0] >= type_counts[2]
            {
                CloneType::Type1
            } else if type_counts[1] >= type_counts[2] {
                CloneType::Type2
            } else {
                CloneType::Type3
            };

            groups.push(CloneGroup {
                id: 0,
                instances: member_indices
                    .into_iter()
                    .map(|idx| instances[idx].clone())
                    .collect(),
                canonical_index: Some(0),
                clone_type,
                avg_similarity: if similarity_count == 0 {
                    0.0
                } else {
                    similarity_total / similarity_count as f64
                },
            });
        }

        groups.sort_unstable_by(|left, right| {
            let left_inst = left.instances.first();
            let right_inst = right.instances.first();
            left_inst
                .map(|inst| &inst.file)
                .cmp(&right_inst.map(|inst| &inst.file))
                .then_with(|| {
                    left_inst
                        .map(|inst| inst.start_byte)
                        .cmp(&right_inst.map(|inst| inst.start_byte))
                })
        });

        for (idx, group) in groups.iter_mut().enumerate() {
            group.id = idx + 1;
        }

        groups
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
