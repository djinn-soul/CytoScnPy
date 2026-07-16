use crate::clones::{CloneGroup, CloneInstance, ClonePair, CloneType};
use rustc_hash::{FxHashMap, FxHashSet};
use std::cmp::Ordering;
use std::path::Path;

mod split;
use split::split_dense_component;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct InstanceKey<'a> {
    file: &'a Path,
    start_byte: usize,
    end_byte: usize,
    start_line: usize,
    end_line: usize,
}

#[derive(Clone, Copy, Debug)]
struct CloneEdge {
    idx_a: usize,
    idx_b: usize,
    clone_type: CloneType,
    similarity: f64,
}

struct IndexedPairs {
    instances: Vec<CloneInstance>,
    edges: Vec<CloneEdge>,
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
            Ordering::Less => self.parent[root_a] = root_b,
            Ordering::Greater => self.parent[root_b] = root_a,
            Ordering::Equal => {
                self.parent[root_b] = root_a;
                self.rank[root_a] = self.rank[root_a].saturating_add(1);
            }
        }
    }
}

pub(super) fn group_clones(pairs: &[ClonePair]) -> Vec<CloneGroup> {
    if pairs.is_empty() {
        return Vec::new();
    }

    let indexed = index_pairs(pairs);
    let members_by_root = connected_components(indexed.instances.len(), &indexed.edges);
    let mut groups = build_groups(members_by_root, &indexed.instances, &indexed.edges);
    sort_groups(&mut groups);
    assign_group_ids(&mut groups);
    groups
}

fn index_pairs(pairs: &[ClonePair]) -> IndexedPairs {
    let mut index_by_key = FxHashMap::default();
    let mut instances = Vec::new();
    let mut edges = Vec::with_capacity(pairs.len());

    for pair in pairs {
        let idx_a = instance_index(&mut index_by_key, &mut instances, &pair.instance_a);
        let idx_b = instance_index(&mut index_by_key, &mut instances, &pair.instance_b);
        edges.push(CloneEdge {
            idx_a,
            idx_b,
            clone_type: pair.clone_type,
            similarity: pair.similarity,
        });
    }

    IndexedPairs { instances, edges }
}

fn instance_index<'a>(
    index_by_key: &mut FxHashMap<InstanceKey<'a>, usize>,
    instances: &mut Vec<CloneInstance>,
    instance: &'a CloneInstance,
) -> usize {
    *index_by_key
        .entry(instance_key(instance))
        .or_insert_with(|| {
            let idx = instances.len();
            instances.push(instance.clone());
            idx
        })
}

fn instance_key(instance: &CloneInstance) -> InstanceKey<'_> {
    InstanceKey {
        file: instance.file.as_path(),
        start_byte: instance.start_byte,
        end_byte: instance.end_byte,
        start_line: instance.start_line,
        end_line: instance.end_line,
    }
}

fn connected_components(
    instance_count: usize,
    edges: &[CloneEdge],
) -> FxHashMap<usize, Vec<usize>> {
    let mut union_find = UnionFind::new(instance_count);
    for edge in edges {
        union_find.union(edge.idx_a, edge.idx_b);
    }

    let mut members_by_root: FxHashMap<usize, Vec<usize>> = FxHashMap::default();
    for idx in 0..instance_count {
        let root = union_find.find(idx);
        members_by_root.entry(root).or_default().push(idx);
    }
    members_by_root
}

fn build_groups(
    mut members_by_root: FxHashMap<usize, Vec<usize>>,
    instances: &[CloneInstance],
    edges: &[CloneEdge],
) -> Vec<CloneGroup> {
    let mut groups = Vec::with_capacity(members_by_root.len());
    let mut roots: Vec<usize> = members_by_root.keys().copied().collect();
    roots.sort_unstable();

    for root in roots {
        let member_indices = members_by_root.remove(&root).unwrap_or_default();
        if member_indices.len() < 2 {
            continue;
        }

        for mut cluster in split_dense_component(&member_indices, edges) {
            if cluster.len() < 2 {
                continue;
            }
            groups.push(build_group(cluster.as_mut_slice(), instances, edges));
        }
    }
    groups
}

fn build_group(
    member_indices: &mut [usize],
    instances: &[CloneInstance],
    edges: &[CloneEdge],
) -> CloneGroup {
    member_indices.sort_unstable_by(|a, b| compare_instances(&instances[*a], &instances[*b]));
    let stats = group_stats(member_indices, edges);

    CloneGroup {
        id: 0,
        instances: member_indices
            .iter()
            .map(|idx| instances[*idx].clone())
            .collect(),
        canonical_index: Some(0),
        clone_type: majority_clone_type(stats.type_counts),
        avg_similarity: stats.average_similarity(),
    }
}

fn compare_instances(left: &CloneInstance, right: &CloneInstance) -> Ordering {
    left.file
        .cmp(&right.file)
        .then_with(|| left.start_byte.cmp(&right.start_byte))
        .then_with(|| left.end_byte.cmp(&right.end_byte))
        .then_with(|| left.start_line.cmp(&right.start_line))
        .then_with(|| left.end_line.cmp(&right.end_line))
}

#[derive(Default)]
struct GroupStats {
    similarity_total: f64,
    similarity_count: usize,
    type_counts: [usize; 3],
}

impl GroupStats {
    fn record(&mut self, edge: &CloneEdge) {
        self.similarity_total += edge.similarity;
        self.similarity_count += 1;
        self.type_counts[type_index(edge.clone_type)] += 1;
    }

    fn average_similarity(&self) -> f64 {
        if self.similarity_count == 0 {
            0.0
        } else {
            self.similarity_total / self.similarity_count as f64
        }
    }
}

fn group_stats(member_indices: &[usize], edges: &[CloneEdge]) -> GroupStats {
    let member_set: FxHashSet<usize> = member_indices.iter().copied().collect();
    let mut stats = GroupStats::default();

    for edge in edges {
        if member_set.contains(&edge.idx_a) && member_set.contains(&edge.idx_b) {
            stats.record(edge);
        }
    }
    stats
}

fn type_index(clone_type: CloneType) -> usize {
    match clone_type {
        CloneType::Type1 => 0,
        CloneType::Type2 => 1,
        CloneType::Type3 => 2,
    }
}

fn majority_clone_type(type_counts: [usize; 3]) -> CloneType {
    if type_counts[0] >= type_counts[1] && type_counts[0] >= type_counts[2] {
        CloneType::Type1
    } else if type_counts[1] >= type_counts[2] {
        CloneType::Type2
    } else {
        CloneType::Type3
    }
}

fn sort_groups(groups: &mut [CloneGroup]) {
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
}

fn assign_group_ids(groups: &mut [CloneGroup]) {
    for (idx, group) in groups.iter_mut().enumerate() {
        group.id = idx + 1;
    }
}
