use super::*;
use crate::clones::types::{CloneInstance, CloneType, NodeKind};
use std::path::PathBuf;

fn make_pair(similarity: f64, clone_type: CloneType, edit_distance: usize) -> ClonePair {
    let instance = |file| CloneInstance {
        file: PathBuf::from(file),
        start_line: 1,
        end_line: 10,
        start_byte: 0,
        end_byte: 100,
        normalized_hash: 0,
        name: None,
        node_kind: NodeKind::Function,
    };
    ClonePair {
        instance_a: instance("a.py"),
        instance_b: instance("b.py"),
        similarity,
        clone_type,
        edit_distance,
    }
}

#[test]
fn test_high_confidence_auto_fix() {
    let scorer = ConfidenceScorer::default();
    let pair = make_pair(0.98, CloneType::Type1, 0);
    let context = FixContext {
        structural_match_verified: true,
        is_idempotent: true,
        ..Default::default()
    };

    let result = scorer.score(&pair, &context);
    assert_eq!(result.decision, FixDecision::AutoFix);
    assert!(result.score >= 90);
}

#[test]
fn test_low_confidence_suppress() {
    let scorer = ConfidenceScorer::default();
    let pair = make_pair(0.65, CloneType::Type3, 15);
    let context = FixContext {
        is_test_file: true,
        control_flow_differs: true,
        ..Default::default()
    };

    let result = scorer.score(&pair, &context);
    assert_eq!(result.decision, FixDecision::Suppress);
}
