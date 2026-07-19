use super::CloneDetector;
#[cfg(feature = "cfg")]
use crate::clones::{parser, ClonePair};

impl CloneDetector {
    /// Validate clone pairs using CFG behavioral analysis
    ///
    /// Filters out pairs where the control flow structure differs significantly.
    /// Only applies to function-level clones (functions have meaningful CFG).
    #[cfg(feature = "cfg")]
    pub(super) fn validate_with_cfg(
        &self,
        pairs: Vec<ClonePair>,
        subtrees: &[parser::Subtree],
    ) -> Vec<ClonePair> {
        let subtree_map: std::collections::HashMap<_, _> = subtrees
            .iter()
            .enumerate()
            .map(|(index, subtree)| ((subtree.file.clone(), subtree.start_byte), index))
            .collect();
        let threshold = self.config.cfg_similarity_threshold;

        pairs
            .into_iter()
            .filter(|pair| {
                let key_a = (pair.instance_a.file.clone(), pair.instance_a.start_byte);
                let key_b = (pair.instance_b.file.clone(), pair.instance_b.start_byte);
                let (Some(&index_a), Some(&index_b)) =
                    (subtree_map.get(&key_a), subtree_map.get(&key_b))
                else {
                    return false;
                };
                cfg_subtrees_match(&subtrees[index_a], &subtrees[index_b], threshold)
            })
            .collect()
    }

    #[cfg(feature = "cfg")]
    pub(super) fn validate_with_cfg_from_paths(
        &self,
        pairs: Vec<ClonePair>,
        min_lines: usize,
    ) -> Vec<ClonePair> {
        let threshold = self.config.cfg_similarity_threshold;

        pairs
            .into_iter()
            .filter(|pair| {
                let Some(subtree_a) = load_subtree(&pair.instance_a, min_lines) else {
                    return false;
                };
                let Some(subtree_b) = load_subtree(&pair.instance_b, min_lines) else {
                    return false;
                };

                cfg_subtrees_match(&subtree_a, &subtree_b, threshold)
            })
            .collect()
    }
}

#[cfg(feature = "cfg")]
fn dedent_definition(source: &str) -> String {
    let indent = source
        .lines()
        .find(|line| !line.trim().is_empty())
        .map(|line| &line[..line.len() - line.trim_start().len()])
        .unwrap_or_default();
    if indent.is_empty() {
        return source.to_owned();
    }
    source
        .lines()
        .map(|line| line.strip_prefix(indent).unwrap_or(line))
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(feature = "cfg")]
fn cfg_subtrees_match(
    subtree_a: &parser::Subtree,
    subtree_b: &parser::Subtree,
    threshold: f64,
) -> bool {
    use crate::cfg::Cfg;
    use parser::SubtreeType;

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
    let cfg_a = Cfg::from_source(&dedent_definition(&subtree_a.source_slice), name_a);
    let cfg_b = Cfg::from_source(&dedent_definition(&subtree_b.source_slice), name_b);
    matches!((cfg_a, cfg_b), (Some(a), Some(b)) if a.similarity_score(&b) >= threshold)
}

#[cfg(feature = "cfg")]
fn load_subtree(
    instance: &crate::clones::CloneInstance,
    min_lines: usize,
) -> Option<parser::Subtree> {
    let source = std::fs::read_to_string(&instance.file).ok()?;
    parser::extract_subtrees_with_min_lines(&source, &instance.file, min_lines)
        .ok()?
        .into_iter()
        .find(|subtree| subtree.start_byte == instance.start_byte)
}

#[cfg(all(test, feature = "cfg"))]
mod tests {
    use super::{dedent_definition, CloneDetector};
    use crate::clones::{CloneConfig, CloneInstance, ClonePair, CloneType, NodeKind};
    use std::path::PathBuf;

    #[test]
    fn dedents_class_method_slice() {
        let source = "    @decorator\n    def method(self):\n        return 1";
        assert_eq!(
            dedent_definition(source),
            "@decorator\ndef method(self):\n    return 1"
        );
    }

    #[test]
    fn cfg_validation_rejects_pairs_without_subtree_metadata() {
        let instance = |file| CloneInstance {
            file: PathBuf::from(file),
            start_line: 1,
            end_line: 5,
            start_byte: 0,
            end_byte: 50,
            normalized_hash: 1,
            name: Some("function".to_owned()),
            node_kind: NodeKind::Function,
        };
        let pair = ClonePair {
            instance_a: instance("left.py"),
            instance_b: instance("right.py"),
            similarity: 1.0,
            clone_type: CloneType::Type1,
            edit_distance: 0,
        };
        let detector = CloneDetector::with_config(CloneConfig::default().with_cfg_validation(true))
            .expect("valid clone config");
        assert!(detector
            .validate_with_cfg_from_paths(vec![pair], 1)
            .is_empty());
    }
}
