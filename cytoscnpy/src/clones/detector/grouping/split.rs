use super::CloneEdge;
use rustc_hash::FxHashSet;

/// Preserve a dense component as one group. Sparse components become
/// overlapping edge groups so every verified relationship remains visible
/// without claiming that transitively connected nodes are mutual clones.
pub(super) fn split_dense_component(
    member_indices: &[usize],
    edges: &[CloneEdge],
) -> Vec<Vec<usize>> {
    if member_indices.len() < crate::constants::CLONE_GROUP_SPLIT_MIN_SIZE
        || member_indices.len() > crate::constants::CLONE_GROUP_SPLIT_MAX_SIZE
    {
        return vec![member_indices.to_vec()];
    }

    let members: FxHashSet<_> = member_indices.iter().copied().collect();
    let mut component_edges = Vec::new();
    let mut seen = FxHashSet::default();
    for edge in edges {
        if !members.contains(&edge.idx_a) || !members.contains(&edge.idx_b) {
            continue;
        }
        let pair = if edge.idx_a < edge.idx_b {
            (edge.idx_a, edge.idx_b)
        } else {
            (edge.idx_b, edge.idx_a)
        };
        if seen.insert(pair) {
            component_edges.push(pair);
        }
    }

    let n = member_indices.len();
    let possible_pairs = n * (n - 1) / 2;
    #[allow(clippy::cast_precision_loss)]
    let density = component_edges.len() as f64 / possible_pairs as f64;
    if density >= crate::constants::CLONE_GROUP_MIN_DENSITY {
        return vec![member_indices.to_vec()];
    }

    component_edges.sort_unstable();
    component_edges
        .into_iter()
        .map(|(left, right)| vec![left, right])
        .collect()
}
