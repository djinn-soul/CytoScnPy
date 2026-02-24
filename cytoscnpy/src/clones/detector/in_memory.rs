use super::{CloneDetectionResult, CloneDetector};
use crate::clones::hasher;
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
        if let Ok(subtrees) = parser::extract_subtrees(source, path) {
            all_subtrees.extend(subtrees);
        }
    }

    let id_normalizer = Normalizer::for_clone_type(CloneType::Type2);
    let hasher = hasher::LshHasher::new(detector.config.lsh_bands, detector.config.lsh_rows);

    let filtered_subtrees: Vec<&parser::Subtree> = all_subtrees
        .iter()
        .filter(|s| {
            let line_count = s.end_line.saturating_sub(s.start_line).saturating_add(1);
            line_count >= min_lines && line_count <= max_lines
        })
        .collect();

    let fingerprints: Vec<_> = filtered_subtrees
        .iter()
        .map(|s| {
            let normalized = id_normalizer.normalize(s);
            let mut struct_hasher = rustc_hash::FxHasher::default();
            for kind in normalized.kind_sequence() {
                use std::hash::Hash;
                kind.hash(&mut struct_hasher);
            }
            parser::CloneFingerprint {
                file: s.file.clone(),
                start_byte: s.start_byte,
                end_byte: s.end_byte,
                start_line: s.start_line,
                end_line: s.end_line,
                name: s.name.clone(),
                node_type: s.node_type,
                lsh_signature: hasher.signature(&normalized),
                structural_hash: struct_hasher.finish(),
            }
        })
        .collect();

    let candidates = hasher.find_candidates_from_fingerprints(&fingerprints);
    let similarity_calc = TreeSimilarity::default();
    let mut pairs = Vec::new();

    for (i, j) in candidates {
        let id_a = id_normalizer.normalize(filtered_subtrees[i]);
        let id_b = id_normalizer.normalize(filtered_subtrees[j]);
        let id_sim = similarity_calc.similarity(&id_a, &id_b);

        if id_sim >= detector.config.min_similarity {
            let clone_type = similarity_calc.classify_by_similarity(id_sim);
            if !detector.is_type_enabled(clone_type) {
                continue;
            }

            pairs.push(crate::clones::ClonePair {
                instance_a: fingerprints[i].to_instance(),
                instance_b: fingerprints[j].to_instance(),
                similarity: id_sim,
                clone_type,
                edit_distance: similarity_calc.edit_distance(&id_a, &id_b),
            });
        }
    }

    let groups = detector.group_clones(&pairs);
    CloneDetectionResult {
        pairs,
        groups: groups.clone(),
        summary: CloneSummary::from_groups(&groups),
    }
}
