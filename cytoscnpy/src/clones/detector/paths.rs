use super::{CloneDetectionResult, CloneDetector};
use crate::clones::hasher;
use crate::clones::parser;
use crate::clones::{ClonePair, CloneSummary, CloneType, Normalizer, TreeSimilarity};
use std::hash::Hasher;
use std::path::PathBuf;

pub(super) fn detect_from_paths(
    detector: &CloneDetector,
    paths: &[PathBuf],
) -> CloneDetectionResult {
    let fingerprints = extract_fingerprints(detector, paths);
    let hasher = hasher::LshHasher::new(detector.config.lsh_bands, detector.config.lsh_rows);
    let candidates = hasher.find_candidates_from_fingerprints(&fingerprints);
    find_and_group_clones(detector, &fingerprints, candidates)
}

fn extract_fingerprints(
    detector: &CloneDetector,
    paths: &[PathBuf],
) -> Vec<parser::CloneFingerprint> {
    use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};

    if let Some(ref pb) = detector.progress_bar {
        pb.set_length(paths.len() as u64);
        pb.set_position(0);
        pb.set_message("Extracting clone fingerprints...");
        pb.set_style(
            indicatif::ProgressStyle::default_bar()
                .template("{spinner:.cyan} [{bar:40.cyan/blue}] {percent}% - Analyzing file fingerprints...")
                .unwrap_or_else(|_| indicatif::ProgressStyle::default_bar())
                .progress_chars("█▓░"),
        );
    }

    let mut fingerprints: Vec<parser::CloneFingerprint> = Vec::new();
    let id_normalizer = Normalizer::for_clone_type(CloneType::Type2);
    let hasher = hasher::LshHasher::new(detector.config.lsh_bands, detector.config.lsh_rows);
    let min_lines = detector.config.min_lines;
    let max_lines = detector.config.max_lines;

    for chunk in paths.chunks(crate::constants::CHUNK_SIZE) {
        let chunk_fingerprints: Vec<parser::CloneFingerprint> = chunk
            .par_iter()
            .filter_map(|path| {
                if crate::CANCELLED.load(std::sync::atomic::Ordering::Relaxed) {
                    return None;
                }
                let source = std::fs::read_to_string(path).ok()?;
                let subtrees = parser::extract_subtrees(&source, path).ok()?;

                if let Some(ref pb) = detector.progress_bar {
                    pb.inc(1);
                }

                Some(
                    subtrees
                        .into_iter()
                        .filter_map(|s| {
                            let line_count =
                                s.end_line.saturating_sub(s.start_line).saturating_add(1);
                            if line_count < min_lines || line_count > max_lines {
                                return None;
                            }

                            let normalized = id_normalizer.normalize(&s);
                            let lsh_signature = hasher.signature(&normalized);
                            let mut struct_hasher = rustc_hash::FxHasher::default();
                            for kind in normalized.kind_sequence() {
                                use std::hash::Hash;
                                kind.hash(&mut struct_hasher);
                            }

                            Some(parser::CloneFingerprint {
                                file: s.file,
                                start_byte: s.start_byte,
                                end_byte: s.end_byte,
                                start_line: s.start_line,
                                end_line: s.end_line,
                                name: s.name,
                                node_type: s.node_type,
                                lsh_signature,
                                structural_hash: struct_hasher.finish(),
                            })
                        })
                        .collect::<Vec<_>>(),
                )
            })
            .flatten()
            .collect();

        fingerprints.extend(chunk_fingerprints);
    }
    fingerprints
}

fn find_and_group_clones(
    detector: &CloneDetector,
    fingerprints: &[parser::CloneFingerprint],
    candidates: Vec<(usize, usize)>,
) -> CloneDetectionResult {
    let similarity_calc = TreeSimilarity::default();
    let mut pairs = Vec::new();
    let mut subtree_cache: std::collections::HashMap<PathBuf, Vec<parser::Subtree>> =
        std::collections::HashMap::new();
    let total_candidates = candidates.len();

    if let Some(ref pb) = detector.progress_bar {
        pb.set_length(total_candidates as u64);
        pb.set_position(0);
        pb.set_message("");
        pb.set_style(
            indicatif::ProgressStyle::default_bar()
                .template(
                    "{spinner:.cyan} [{bar:40.cyan/blue}] {percent}% - Checking code similarity...",
                )
                .unwrap_or_else(|_| indicatif::ProgressStyle::default_bar())
                .progress_chars("█▓░"),
        );
    }

    for (idx, (i, j)) in candidates.into_iter().enumerate() {
        if let Some(ref pb) = detector.progress_bar {
            if idx % 100 == 0 || idx == total_candidates.saturating_sub(1) {
                pb.set_position(idx as u64);
            }
        }

        let fp_a = &fingerprints[i];
        let fp_b = &fingerprints[j];
        ensure_subtrees_loaded(&mut subtree_cache, &fp_a.file);
        ensure_subtrees_loaded(&mut subtree_cache, &fp_b.file);

        let sub_a = subtree_cache
            .get(&fp_a.file)
            .and_then(|st| st.iter().find(|s| s.start_byte == fp_a.start_byte));
        let sub_b = subtree_cache
            .get(&fp_b.file)
            .and_then(|st| st.iter().find(|s| s.start_byte == fp_b.start_byte));

        let (Some(sub_a), Some(sub_b)) = (sub_a, sub_b) else {
            continue;
        };

        if let Some(pair) = build_pair(detector, &similarity_calc, fp_a, fp_b, sub_a, sub_b) {
            pairs.push(pair);
        }
    }

    if let Some(ref pb) = detector.progress_bar {
        pb.finish_and_clear();
    }

    let groups = detector.group_clones(&pairs);
    let summary = CloneSummary::from_groups(&groups);
    CloneDetectionResult {
        pairs,
        groups,
        summary,
    }
}

fn ensure_subtrees_loaded(
    cache: &mut std::collections::HashMap<PathBuf, Vec<parser::Subtree>>,
    file: &PathBuf,
) {
    if cache.contains_key(file) {
        return;
    }
    if let Ok(source) = std::fs::read_to_string(file) {
        if let Ok(st) = parser::extract_subtrees(&source, file) {
            cache.insert(file.clone(), st);
        }
    }
}

fn build_pair(
    detector: &CloneDetector,
    similarity_calc: &TreeSimilarity,
    fp_a: &parser::CloneFingerprint,
    fp_b: &parser::CloneFingerprint,
    sub_a: &parser::Subtree,
    sub_b: &parser::Subtree,
) -> Option<ClonePair> {
    let raw_normalizer = Normalizer::for_clone_type(CloneType::Type1);
    let id_normalizer = Normalizer::for_clone_type(CloneType::Type2);
    let raw_a = raw_normalizer.normalize(sub_a);
    let raw_b = raw_normalizer.normalize(sub_b);
    let id_a = id_normalizer.normalize(sub_a);
    let id_b = id_normalizer.normalize(sub_b);

    let raw_sim = similarity_calc.similarity(&raw_a, &raw_b);
    let id_sim = similarity_calc.similarity(&id_a, &id_b);
    if id_sim < detector.config.min_similarity {
        return None;
    }

    let t1 = detector.config.type1_threshold;
    let t2_raw = detector.config.type2_raw_max;
    let clone_type = if raw_sim >= t1 && id_sim >= t1 {
        CloneType::Type1
    } else if id_sim >= t1 && raw_sim < t2_raw {
        CloneType::Type2
    } else if id_sim >= t1 {
        CloneType::Type1
    } else {
        CloneType::Type3
    };

    if !detector.is_type_enabled(clone_type) {
        return None;
    }

    Some(ClonePair {
        instance_a: fp_a.to_instance(),
        instance_b: fp_b.to_instance(),
        similarity: id_sim,
        clone_type,
        edit_distance: similarity_calc.edit_distance(&id_a, &id_b),
    })
}
