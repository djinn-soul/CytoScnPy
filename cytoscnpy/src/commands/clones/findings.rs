use super::suggestions::generate_clone_suggestion;
use crate::clones::{CloneFinding, ClonePair, ConfidenceScorer, FixContext, FixDecision};
use rayon::prelude::*;
use std::collections::HashMap;
use std::path::PathBuf;

/// Helper to generate findings from clone pairs.
#[must_use]
pub fn generate_clone_findings(
    pairs: &[ClonePair],
    all_files: &[(PathBuf, String)],
    with_cst: bool,
) -> Vec<CloneFinding> {
    generate_clone_findings_with_thresholds(pairs, all_files, with_cst, 90, 60)
}

/// Generates clone findings with explicit confidence thresholds.
#[must_use]
pub fn generate_clone_findings_with_thresholds(
    pairs: &[ClonePair],
    all_files: &[(PathBuf, String)],
    with_cst: bool,
    auto_fix_threshold: u8,
    suggest_threshold: u8,
) -> Vec<CloneFinding> {
    #[cfg(not(feature = "cst"))]
    let _ = with_cst;

    #[cfg(feature = "cst")]
    use crate::cst::{AstCstMapper, CstParser};

    #[cfg(feature = "cst")]
    let mappers: HashMap<&PathBuf, AstCstMapper> = if with_cst {
        all_files
            .iter()
            .filter_map(|(p, c)| {
                CstParser::new()
                    .ok()
                    .and_then(|mut parser| parser.parse(c).ok())
                    .map(|tree| (p, AstCstMapper::new(tree)))
            })
            .collect()
    } else {
        HashMap::new()
    };

    let scorer = ConfidenceScorer::new(auto_fix_threshold, suggest_threshold);
    let mut findings: Vec<CloneFinding> = pairs
        .par_iter()
        .flat_map(|pair| {
            let calc_conf = |inst: &crate::clones::CloneInstance| {
                let ctx = FixContext {
                    is_test_file: crate::utils::is_test_path(&inst.file.to_string_lossy()),
                    same_file: pair.is_same_file(),
                    ..FixContext::default()
                };

                #[cfg(feature = "cst")]
                let ctx = {
                    let mut ctx = ctx;
                    if with_cst {
                        if let Some(mapper) = mappers.get(&inst.file) {
                            ctx.has_interleaved_comments =
                                mapper.has_interleaved_comments(inst.start_byte, inst.end_byte);
                            ctx.deeply_nested =
                                mapper.is_deeply_nested(inst.start_byte, inst.end_byte);
                        }
                    }
                    ctx
                };

                #[cfg(not(feature = "cst"))]
                let _ = inst;

                let confidence = scorer.score(pair, &ctx);
                (confidence.score, confidence.decision)
            };

            let (canonical_score, canonical_decision) = calc_conf(&pair.instance_a);
            let (duplicate_score, duplicate_decision) = calc_conf(&pair.instance_b);
            [
                (false, canonical_score, canonical_decision),
                (true, duplicate_score, duplicate_decision),
            ]
            .into_iter()
            .filter(|(_, _, decision)| *decision != FixDecision::Suppress)
            .map(|(duplicate, score, _)| CloneFinding::from_pair(pair, duplicate, score))
            .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    for finding in &mut findings {
        let name = finding.name.as_deref().unwrap_or("<anonymous>");
        finding.suggestion = Some(generate_clone_suggestion(
            finding.clone_type,
            finding.node_kind,
            name,
            finding.similarity,
        ));
    }

    let mut best_by_location: HashMap<(String, usize, usize, usize), CloneFinding> = HashMap::new();
    for finding in findings {
        let key = (
            finding.file.display().to_string(),
            finding.line,
            finding.start_byte,
            finding.end_byte,
        );
        match best_by_location.entry(key) {
            std::collections::hash_map::Entry::Vacant(e) => {
                e.insert(finding);
            }
            std::collections::hash_map::Entry::Occupied(mut e) => {
                let existing = e.get();
                let prefer_reportable_duplicate = finding.is_duplicate && !existing.is_duplicate;
                let same_role_with_better_match = finding.is_duplicate == existing.is_duplicate
                    && finding.similarity > existing.similarity;
                if prefer_reportable_duplicate || same_role_with_better_match {
                    e.insert(finding);
                }
            }
        }
    }

    let file_contents: HashMap<_, _> = all_files.iter().map(|(p, c)| (p, c)).collect();
    let mut findings: Vec<_> = best_by_location
        .into_values()
        .filter(|finding| {
            if let Some(content) = file_contents.get(&finding.file) {
                if let Some(line) = content.lines().nth(finding.line.saturating_sub(1)) {
                    if crate::utils::get_line_suppression(line).is_some() {
                        return false;
                    }
                }
            }
            true
        })
        .collect();
    findings.sort_unstable_by(|left, right| {
        left.file
            .cmp(&right.file)
            .then_with(|| left.line.cmp(&right.line))
            .then_with(|| left.start_byte.cmp(&right.start_byte))
            .then_with(|| left.end_byte.cmp(&right.end_byte))
            .then_with(|| left.related_clone.file.cmp(&right.related_clone.file))
            .then_with(|| left.related_clone.line.cmp(&right.related_clone.line))
    });
    findings
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clones::{CloneInstance, CloneType, NodeKind};

    fn instance(file: &str, start_line: usize, start_byte: usize) -> CloneInstance {
        CloneInstance {
            file: PathBuf::from(file),
            start_line,
            end_line: start_line + 4,
            start_byte,
            end_byte: start_byte + 20,
            normalized_hash: 0,
            name: None,
            node_kind: NodeKind::Function,
        }
    }

    #[test]
    fn test_file_penalty_is_scored_per_instance() {
        let pair = ClonePair {
            instance_a: instance("src/module.py", 1, 0),
            instance_b: instance("tests/test_module.py", 1, 30),
            similarity: 0.85,
            clone_type: CloneType::Type3,
            edit_distance: 5,
        };
        let findings = generate_clone_findings_with_thresholds(&[pair], &[], false, 90, 40);

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].file, PathBuf::from("src/module.py"));
    }

    #[test]
    fn preserves_distinct_blocks_on_the_same_line() {
        let pairs = [
            ClonePair {
                instance_a: instance("src/module.py", 1, 0),
                instance_b: instance("src/other.py", 10, 30),
                similarity: 1.0,
                clone_type: CloneType::Type1,
                edit_distance: 0,
            },
            ClonePair {
                instance_a: instance("src/module.py", 1, 40),
                instance_b: instance("src/third.py", 20, 70),
                similarity: 1.0,
                clone_type: CloneType::Type1,
                edit_distance: 0,
            },
        ];
        let findings = generate_clone_findings(&pairs, &[], false);

        assert_eq!(
            findings
                .iter()
                .filter(|f| f.file == std::path::Path::new("src/module.py"))
                .count(),
            2
        );
    }
}
