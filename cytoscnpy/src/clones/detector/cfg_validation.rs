use super::CloneDetector;
use crate::clones::{parser, ClonePair};
use std::path::PathBuf;

impl CloneDetector {
    /// Validate clone pairs using CFG behavioral analysis
    ///
    /// Filters out pairs where the control flow structure differs significantly.
    /// Only applies to function-level clones (functions have meaningful CFG).
    #[cfg(feature = "cfg")]
    #[allow(dead_code)] // Optional validation path, enabled for future CLI wiring.
    #[allow(clippy::unused_self)]
    pub(super) fn validate_with_cfg(
        &self,
        pairs: Vec<ClonePair>,
        subtrees: &[parser::Subtree],
    ) -> Vec<ClonePair> {
        use crate::cfg::Cfg;
        use parser::SubtreeType;

        let subtree_map: std::collections::HashMap<(PathBuf, usize), usize> = subtrees
            .iter()
            .enumerate()
            .map(|(i, s)| ((s.file.clone(), s.start_byte), i))
            .collect();

        pairs
            .into_iter()
            .filter(|pair| {
                let key_a = (pair.instance_a.file.clone(), pair.instance_a.start_byte);
                let key_b = (pair.instance_b.file.clone(), pair.instance_b.start_byte);

                let (Some(&idx_a), Some(&idx_b)) =
                    (subtree_map.get(&key_a), subtree_map.get(&key_b))
                else {
                    return true;
                };

                let subtree_a = &subtrees[idx_a];
                let subtree_b = &subtrees[idx_b];

                let is_function_a = matches!(
                    subtree_a.node_type,
                    SubtreeType::Function | SubtreeType::AsyncFunction | SubtreeType::Method
                );
                let is_function_b = matches!(
                    subtree_b.node_type,
                    SubtreeType::Function | SubtreeType::AsyncFunction | SubtreeType::Method
                );
                if !is_function_a || !is_function_b {
                    return true;
                }

                let name_a = subtree_a.name.as_deref().unwrap_or("func");
                let name_b = subtree_b.name.as_deref().unwrap_or("func");

                let cfg_a = Cfg::from_source(&subtree_a.source_slice, name_a);
                let cfg_b = Cfg::from_source(&subtree_b.source_slice, name_b);

                match (cfg_a, cfg_b) {
                    (Some(a), Some(b)) => a.similarity_score(&b) >= 0.7,
                    _ => true,
                }
            })
            .collect()
    }
}
