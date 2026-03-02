use super::generate_clone_suggestion;
use crate::clones::{CloneFinding, ClonePair, ConfidenceScorer, FixContext};
use rayon::prelude::*;
use std::collections::HashMap;
use std::path::PathBuf;

/// Helper to generate findings from clone pairs.
#[must_use]
pub fn generate_clone_findings(
    pairs: &[ClonePair],
    all_files: &[(PathBuf, String)],
    #[allow(unused_variables)] with_cst: bool,
) -> Vec<CloneFinding> {
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

    let scorer = ConfidenceScorer::default();
    let mut findings: Vec<CloneFinding> = pairs
        .par_iter()
        .flat_map(|pair| {
            #[allow(unused_variables)]
            let calc_conf = |inst: &crate::clones::CloneInstance| -> u8 {
                #[allow(unused_mut)]
                let mut ctx = FixContext {
                    same_file: pair.is_same_file(),
                    ..FixContext::default()
                };

                #[cfg(feature = "cst")]
                if with_cst {
                    if let Some(mapper) = mappers.get(&inst.file) {
                        ctx.has_interleaved_comments =
                            mapper.has_interleaved_comments(inst.start_byte, inst.end_byte);
                        ctx.deeply_nested = mapper.is_deeply_nested(inst.start_byte, inst.end_byte);
                    }
                }

                scorer.score(pair, &ctx).score
            };

            vec![
                CloneFinding::from_pair(pair, false, calc_conf(&pair.instance_a)),
                CloneFinding::from_pair(pair, true, calc_conf(&pair.instance_b)),
            ]
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

    let mut best_by_location: HashMap<(String, usize), CloneFinding> = HashMap::new();
    for finding in findings {
        let key = (finding.file.display().to_string(), finding.line);
        match best_by_location.entry(key) {
            std::collections::hash_map::Entry::Vacant(e) => {
                e.insert(finding);
            }
            std::collections::hash_map::Entry::Occupied(mut e) => {
                if finding.similarity > e.get().similarity {
                    e.insert(finding);
                }
            }
        }
    }

    let file_contents: HashMap<_, _> = all_files.iter().map(|(p, c)| (p, c)).collect();
    best_by_location
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
        .collect()
}
