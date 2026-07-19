use super::cache::{PairSubtreeCache, PreparedSubtree};
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
    let hasher = hasher::LshHasher::new(detector.config.lsh_bands, detector.config.lsh_rows)
        .with_limits(
            detector.config.lsh_boilerplate_threshold,
            detector.config.lsh_max_candidates,
        );
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
    let hasher = hasher::LshHasher::new(detector.config.lsh_bands, detector.config.lsh_rows)
        .with_limits(
            detector.config.lsh_boilerplate_threshold,
            detector.config.lsh_max_candidates,
        );
    let min_lines = detector.config.min_lines;
    let max_lines = detector.config.max_lines;

    for chunk in paths.chunks(crate::constants::CHUNK_SIZE) {
        let chunk_fingerprints: Vec<parser::CloneFingerprint> = chunk
            .par_iter()
            .filter_map(|path| {
                if crate::CANCELLED.load(std::sync::atomic::Ordering::Relaxed) {
                    return None;
                }
                if !detector.should_process_path(path) {
                    return None;
                }
                if let Some(ref pb) = detector.progress_bar {
                    pb.inc(1);
                }
                let source = std::fs::read_to_string(path).ok()?;
                let subtrees =
                    parser::extract_subtrees_with_min_lines(&source, path, min_lines).ok()?;

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
    let mut subtree_cache = PairSubtreeCache::default();
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
        if crate::CANCELLED.load(std::sync::atomic::Ordering::Relaxed) {
            break;
        }
        if let Some(ref pb) = detector.progress_bar {
            if idx % 100 == 0 || idx == total_candidates.saturating_sub(1) {
                pb.set_position((idx + 1) as u64);
            }
        }

        let fp_a = &fingerprints[i];
        let fp_b = &fingerprints[j];
        subtree_cache.load(&fp_a.file, detector.config.min_lines);
        subtree_cache.load(&fp_b.file, detector.config.min_lines);

        let sub_a = subtree_cache.find(&fp_a.file, fp_a.start_byte);
        let sub_b = subtree_cache.find(&fp_b.file, fp_b.start_byte);

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

    #[cfg(feature = "cfg")]
    if detector.config.cfg_validation {
        pairs = detector.validate_with_cfg_from_paths(pairs, detector.config.min_lines);
    }

    let groups = CloneDetector::group_clones(&pairs);
    let summary = CloneSummary::from_groups(&groups);
    CloneDetectionResult {
        pairs,
        groups,
        summary,
    }
}

fn build_pair(
    detector: &CloneDetector,
    similarity_calc: &TreeSimilarity,
    fp_a: &parser::CloneFingerprint,
    fp_b: &parser::CloneFingerprint,
    sub_a: &PreparedSubtree,
    sub_b: &PreparedSubtree,
) -> Option<ClonePair> {
    let raw_sim = similarity_calc.similarity(&sub_a.raw_tree, &sub_b.raw_tree);
    let (edit_distance, id_sim) =
        similarity_calc.distance_and_similarity(&sub_a.id_tree, &sub_b.id_tree);
    if id_sim < detector.config.min_similarity {
        return None;
    }

    let clone_type = detector.classify_clone(raw_sim, id_sim);

    if !detector.is_type_enabled(clone_type) {
        return None;
    }

    Some(ClonePair {
        instance_a: fp_a.to_instance(),
        instance_b: fp_b.to_instance(),
        similarity: id_sim,
        clone_type,
        edit_distance,
    })
}
