//! Tree similarity calculation for clone detection.
//!
//! Implements tree edit distance and similarity scoring
//! for Type-1/2/3 clone classification.

use crate::clones::normalizer::{NormalizedNode, NormalizedTree};
use crate::clones::types::CloneType;

/// Tree similarity calculator
#[derive(Debug, Clone)]
#[allow(clippy::struct_field_names)] // _cost suffix is intentional for clarity
pub struct TreeSimilarity {
    /// Cost of inserting a node
    pub insert_cost: usize,
    /// Cost of deleting a node
    pub delete_cost: usize,
    /// Cost of updating a node (label change)
    pub update_cost: usize,
}

impl Default for TreeSimilarity {
    fn default() -> Self {
        Self {
            insert_cost: 1,
            delete_cost: 1,
            update_cost: 1,
        }
    }
}

impl TreeSimilarity {
    /// Calculate tree edit distance between two normalized trees
    ///
    /// Uses a simplified Zhang-Shasha style algorithm.
    #[must_use]
    pub fn edit_distance(&self, tree_a: &NormalizedTree, tree_b: &NormalizedTree) -> usize {
        // Flatten trees to node sequences for comparison
        let nodes_a = Self::flatten(&tree_a.nodes);
        let nodes_b = Self::flatten(&tree_b.nodes);

        // Use Levenshtein-like DP on flattened nodes
        self.sequence_edit_distance(&nodes_a, &nodes_b)
    }

    /// Calculate similarity score (0.0 - 1.0)
    #[must_use]
    pub fn similarity(&self, tree_a: &NormalizedTree, tree_b: &NormalizedTree) -> f64 {
        let distance = self.edit_distance(tree_a, tree_b);
        let max_size = tree_a.size().max(tree_b.size());

        if max_size == 0 {
            return 1.0;
        }

        1.0 - (distance as f64 / max_size as f64)
    }

    /// Classify clone type based on similarity
    #[must_use]
    #[allow(clippy::unused_self)] // Method interface for consistency
    pub fn classify_by_similarity(&self, similarity: f64) -> CloneType {
        if similarity >= 0.99 {
            CloneType::Type1
        } else if similarity >= 0.90 {
            CloneType::Type2
        } else {
            CloneType::Type3
        }
    }

    /// Classify clone type by comparing raw vs normalized trees
    #[must_use]
    #[allow(clippy::unused_self)] // Method interface for consistency
    pub fn classify(&self, raw_similarity: f64, normalized_similarity: f64) -> CloneType {
        if raw_similarity >= 0.99 {
            CloneType::Type1
        } else if normalized_similarity >= 0.95 {
            CloneType::Type2
        } else {
            CloneType::Type3
        }
    }

    /// Flatten tree to a sequence of (kind, label) pairs
    fn flatten(nodes: &[NormalizedNode]) -> Vec<(&str, Option<&str>)> {
        let mut result = Vec::new();
        for node in nodes {
            Self::flatten_node(node, &mut result);
        }
        result
    }

    /// Recursively flatten a node
    fn flatten_node<'a>(node: &'a NormalizedNode, result: &mut Vec<(&'a str, Option<&'a str>)>) {
        result.push((&node.kind, node.label.as_deref()));
        for child in &node.children {
            Self::flatten_node(child, result);
        }
    }

    /// Compute edit distance on flattened sequences
    fn sequence_edit_distance(
        &self,
        seq_a: &[(&str, Option<&str>)],
        seq_b: &[(&str, Option<&str>)],
    ) -> usize {
        let m = seq_a.len();
        let n = seq_b.len();

        // DP table
        let mut dp = vec![vec![0usize; n + 1]; m + 1];

        // Base cases
        for i in 0..=m {
            dp[i][0] = i * self.delete_cost;
        }
        for j in 0..=n {
            dp[0][j] = j * self.insert_cost;
        }

        // Fill DP table
        for i in 1..=m {
            for j in 1..=n {
                let node_a = &seq_a[i - 1];
                let node_b = &seq_b[j - 1];

                let cost = if node_a.0 == node_b.0 && node_a.1 == node_b.1 {
                    0 // Identical nodes
                } else if node_a.0 == node_b.0 {
                    self.update_cost // Same kind, different label
                } else {
                    self.update_cost * 2 // Different kind
                };

                dp[i][j] = (dp[i - 1][j - 1] + cost)
                    .min(dp[i - 1][j] + self.delete_cost)
                    .min(dp[i][j - 1] + self.insert_cost);
            }
        }

        dp[m][n]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_tree(nodes: Vec<NormalizedNode>) -> NormalizedTree {
        NormalizedTree { nodes }
    }

    fn node(kind: &str, label: Option<&str>) -> NormalizedNode {
        NormalizedNode {
            kind: kind.to_string(),
            label: label.map(String::from),
            children: vec![],
        }
    }

    #[test]
    fn test_identical_trees_have_zero_distance() {
        let calc = TreeSimilarity::default();
        let tree = make_tree(vec![
            node("if", None),
            node("assign", Some("VAR_0")),
            node("return", None),
        ]);

        assert_eq!(calc.edit_distance(&tree, &tree), 0);
        assert!((calc.similarity(&tree, &tree) - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_different_trees_have_nonzero_distance() {
        let calc = TreeSimilarity::default();
        let tree_a = make_tree(vec![node("if", None), node("return", None)]);
        let tree_b = make_tree(vec![node("for", None), node("break", None)]);

        let distance = calc.edit_distance(&tree_a, &tree_b);
        assert!(distance > 0);
    }

    #[test]
    fn test_similar_trees_have_high_similarity() {
        let calc = TreeSimilarity::default();
        let tree_a = make_tree(vec![
            node("if", None),
            node("assign", Some("VAR_0")),
            node("return", None),
        ]);
        let tree_b = make_tree(vec![
            node("if", None),
            node("assign", Some("VAR_0")),
            node("assign", Some("VAR_1")), // Extra node
            node("return", None),
        ]);

        let similarity = calc.similarity(&tree_a, &tree_b);
        assert!(similarity > 0.7);
        assert!(similarity < 1.0);
    }

    #[test]
    fn test_classify_by_similarity() {
        let calc = TreeSimilarity::default();

        assert_eq!(calc.classify_by_similarity(1.0), CloneType::Type1);
        assert_eq!(calc.classify_by_similarity(0.99), CloneType::Type1);
        assert_eq!(calc.classify_by_similarity(0.95), CloneType::Type2);
        assert_eq!(calc.classify_by_similarity(0.85), CloneType::Type3);
        assert_eq!(calc.classify_by_similarity(0.70), CloneType::Type3);
    }
}
