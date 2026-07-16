use super::{CloneDetectionResult, CloneDetector};
use crate::clones::hasher;
use crate::clones::normalizer::NormalizedTree;
use crate::clones::parser;
use crate::clones::{CloneSummary, CloneType, Normalizer, TreeSimilarity};
use std::hash::Hasher;
use std::path::PathBuf;

pub(super) fn detect_from_memory(
    detector: &CloneDetector,
    files: &[(PathBuf, String)],
) -> CloneDetectionResult {
    let mut all_subtrees = Vec::new();
    let min_lines = detector.config.min_lines;
    let max_lines = detector.config.max_lines;

    for (path, source) in files {
        if !detector.should_process_path(path) {
            continue;
        }
        if let Ok(subtrees) = parser::extract_subtrees_with_min_lines(source, path, min_lines) {
            all_subtrees.extend(subtrees);
        }
    }

    let id_normalizer = Normalizer::for_clone_type(CloneType::Type2);
    let raw_normalizer = Normalizer::for_clone_type(CloneType::Type1);
    let hasher = hasher::LshHasher::new(detector.config.lsh_bands, detector.config.lsh_rows)
        .with_limits(
            detector.config.lsh_boilerplate_threshold,
            detector.config.lsh_max_candidates,
        );

    let filtered_subtrees: Vec<&parser::Subtree> = all_subtrees
        .iter()
        .filter(|s| {
            let line_count = s.end_line.saturating_sub(s.start_line).saturating_add(1);
            line_count >= min_lines && line_count <= max_lines
        })
        .collect();

    let prepared: Vec<(parser::CloneFingerprint, NormalizedTree, NormalizedTree)> =
        filtered_subtrees
            .iter()
            .map(|s| {
                let id_tree = id_normalizer.normalize(s);
                let raw_tree = raw_normalizer.normalize(s);
                let mut struct_hasher = rustc_hash::FxHasher::default();
                for kind in id_tree.kind_sequence() {
                    use std::hash::Hash;
                    kind.hash(&mut struct_hasher);
                }
                let fingerprint = parser::CloneFingerprint {
                    file: s.file.clone(),
                    start_byte: s.start_byte,
                    end_byte: s.end_byte,
                    start_line: s.start_line,
                    end_line: s.end_line,
                    name: s.name.clone(),
                    node_type: s.node_type,
                    lsh_signature: hasher.signature(&id_tree),
                    structural_hash: struct_hasher.finish(),
                };
                (fingerprint, raw_tree, id_tree)
            })
            .collect();
    let fingerprints: Vec<_> = prepared.iter().map(|item| item.0.clone()).collect();

    let candidates = hasher.find_candidates_from_fingerprints(&fingerprints);
    let similarity_calc = TreeSimilarity::default();
    let mut pairs = Vec::new();

    for (i, j) in candidates {
        let (_, raw_a, id_a) = &prepared[i];
        let (_, raw_b, id_b) = &prepared[j];
        let (edit_distance, id_sim) = similarity_calc.distance_and_similarity(id_a, id_b);
        let raw_sim = similarity_calc.similarity(raw_a, raw_b);

        if id_sim >= detector.config.min_similarity {
            let clone_type = detector.classify_clone(raw_sim, id_sim);
            if !detector.is_type_enabled(clone_type) {
                continue;
            }

            pairs.push(crate::clones::ClonePair {
                instance_a: fingerprints[i].to_instance(),
                instance_b: fingerprints[j].to_instance(),
                similarity: id_sim,
                clone_type,
                edit_distance,
            });
        }
    }

    #[cfg(feature = "cfg")]
    if detector.config.cfg_validation {
        pairs = detector.validate_with_cfg(pairs, &all_subtrees);
    }

    let groups = CloneDetector::group_clones(&pairs);
    CloneDetectionResult {
        pairs,
        groups: groups.clone(),
        summary: CloneSummary::from_groups(&groups),
    }
}
