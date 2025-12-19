//! AST normalization for clone comparison.
//!
//! Normalizes code for Type-1/2/3 clone detection by abstracting
//! identifiers, literals, and optionally reordering statements.

use crate::clones::parser::{Subtree, SubtreeNode};
use crate::clones::types::CloneType;
use rustc_hash::FxHashMap;

/// A normalized tree ready for comparison
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedTree {
    /// Root nodes of the normalized tree
    pub nodes: Vec<NormalizedNode>,
}

impl NormalizedTree {
    /// Count total nodes in the tree
    #[must_use]
    pub fn size(&self) -> usize {
        self.nodes.iter().map(NormalizedNode::size).sum()
    }

    /// Get a flat sequence of node kinds for hashing
    #[must_use]
    pub fn kind_sequence(&self) -> Vec<&str> {
        let mut seq = Vec::new();
        for node in &self.nodes {
            node.collect_kinds(&mut seq);
        }
        seq
    }
}

/// A normalized node with abstracted identifiers
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NormalizedNode {
    /// Node kind (e.g., "if", "for", "call")
    pub kind: String,
    /// Normalized label (`VAR_0`, `VAR_1`, CONST, etc.)
    pub label: Option<String>,
    /// Child nodes
    pub children: Vec<NormalizedNode>,
}

impl NormalizedNode {
    /// Count nodes in this subtree
    #[must_use]
    pub fn size(&self) -> usize {
        1 + self
            .children
            .iter()
            .map(NormalizedNode::size)
            .sum::<usize>()
    }

    /// Collect kinds into a sequence
    fn collect_kinds<'a>(&'a self, seq: &mut Vec<&'a str>) {
        seq.push(&self.kind);
        for child in &self.children {
            child.collect_kinds(seq);
        }
    }
}

/// Normalizer configuration
#[derive(Debug, Clone)]
pub struct Normalizer {
    /// Normalize identifier names to `VAR_N`
    pub normalize_identifiers: bool,
    /// Normalize literal values to CONST
    pub normalize_literals: bool,
    /// Sort statements for canonical ordering
    pub canonical_ordering: bool,
}

impl Normalizer {
    /// Create a normalizer for a specific clone type
    #[must_use]
    pub fn for_clone_type(clone_type: CloneType) -> Self {
        match clone_type {
            CloneType::Type1 => Self {
                normalize_identifiers: false,
                normalize_literals: false,
                canonical_ordering: false,
            },
            CloneType::Type2 => Self {
                normalize_identifiers: true,
                normalize_literals: true,
                canonical_ordering: false,
            },
            CloneType::Type3 => Self {
                normalize_identifiers: true,
                normalize_literals: true,
                canonical_ordering: true,
            },
        }
    }

    /// Normalize a subtree for comparison
    #[must_use]
    pub fn normalize(&self, subtree: &Subtree) -> NormalizedTree {
        let mut var_map: FxHashMap<String, usize> = FxHashMap::default();
        let mut var_counter = 0;

        let normalized_nodes: Vec<NormalizedNode> = subtree
            .children
            .iter()
            .map(|node| self.normalize_node(node, &mut var_map, &mut var_counter))
            .collect();

        // Optionally sort for canonical ordering
        let final_nodes = if self.canonical_ordering {
            let mut sorted = normalized_nodes;
            sorted.sort_by(|a, b| a.kind.cmp(&b.kind));
            sorted
        } else {
            normalized_nodes
        };

        NormalizedTree { nodes: final_nodes }
    }

    /// Normalize a slice of nodes (for tests)
    #[must_use]
    pub fn normalize_nodes(&self, nodes: &[SubtreeNode]) -> NormalizedTree {
        let mut var_map: FxHashMap<String, usize> = FxHashMap::default();
        let mut var_counter = 0;

        let normalized_nodes: Vec<NormalizedNode> = nodes
            .iter()
            .map(|node| self.normalize_node(node, &mut var_map, &mut var_counter))
            .collect();

        let final_nodes = if self.canonical_ordering {
            let mut sorted = normalized_nodes;
            sorted.sort_by(|a, b| a.kind.cmp(&b.kind));
            sorted
        } else {
            normalized_nodes
        };

        NormalizedTree { nodes: final_nodes }
    }

    /// Normalize a single node
    fn normalize_node(
        &self,
        node: &SubtreeNode,
        var_map: &mut FxHashMap<String, usize>,
        var_counter: &mut usize,
    ) -> NormalizedNode {
        let label = if self.normalize_identifiers {
            node.label.as_ref().map(|name| {
                // Check if it's a literal (we normalize those too)
                if is_literal_pattern(name) && self.normalize_literals {
                    "CONST".to_owned()
                } else {
                    // Map to VAR_N
                    let idx = *var_map.entry(name.clone()).or_insert_with(|| {
                        let current = *var_counter;
                        *var_counter += 1;
                        current
                    });
                    format!("VAR_{idx}")
                }
            })
        } else {
            node.label.clone()
        };

        let children: Vec<NormalizedNode> = node
            .children
            .iter()
            .map(|child| self.normalize_node(child, var_map, var_counter))
            .collect();

        NormalizedNode {
            kind: node.kind.clone(),
            label,
            children,
        }
    }
}

/// Check if a name looks like a literal value
fn is_literal_pattern(name: &str) -> bool {
    // Numbers, quoted strings, True/False/None
    name.parse::<f64>().is_ok()
        || name.starts_with('"')
        || name.starts_with('\'')
        || matches!(name, "True" | "False" | "None")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type1_preserves_names() {
        let normalizer = Normalizer::for_clone_type(CloneType::Type1);
        let nodes = vec![SubtreeNode {
            kind: "assign".into(),
            label: Some("my_var".into()),
            children: vec![],
        }];

        let normalized = normalizer.normalize_nodes(&nodes);
        assert_eq!(normalized.nodes[0].label.as_deref(), Some("my_var"));
    }

    #[test]
    fn test_type2_normalizes_names() {
        let normalizer = Normalizer::for_clone_type(CloneType::Type2);
        let nodes = vec![SubtreeNode {
            kind: "assign".into(),
            label: Some("my_var".into()),
            children: vec![],
        }];

        let normalized = normalizer.normalize_nodes(&nodes);
        assert_eq!(normalized.nodes[0].label.as_deref(), Some("VAR_0"));
    }

    #[test]
    fn test_same_name_maps_to_same_var() {
        let normalizer = Normalizer::for_clone_type(CloneType::Type2);
        let nodes = vec![
            SubtreeNode {
                kind: "assign".into(),
                label: Some("x".into()),
                children: vec![],
            },
            SubtreeNode {
                kind: "use".into(),
                label: Some("x".into()),
                children: vec![],
            },
        ];

        let normalized = normalizer.normalize_nodes(&nodes);
        assert_eq!(normalized.nodes[0].label, normalized.nodes[1].label);
    }
}
