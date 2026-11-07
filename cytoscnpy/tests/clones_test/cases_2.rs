use super::*;

#[test]
fn test_clone_summary_with_groups() {
    let instances_1 = vec![
        CloneInstance {
            file: PathBuf::from("a.py"),
            start_line: 1,
            end_line: 10,
            start_byte: 0,
            end_byte: 100,
            normalized_hash: 1,
            name: None,
            node_kind: NodeKind::Function,
        },
        CloneInstance {
            file: PathBuf::from("a.py"),
            start_line: 20,
            end_line: 30,
            start_byte: 200,
            end_byte: 300,
            normalized_hash: 2,
            name: None,
            node_kind: NodeKind::Function,
        },
    ];

    let instances_2 = vec![CloneInstance {
        file: PathBuf::from("b.py"),
        start_line: 5,
        end_line: 15,
        start_byte: 50,
        end_byte: 150,
        normalized_hash: 3,
        name: None,
        node_kind: NodeKind::Function,
    }];

    let groups = vec![
        CloneGroup {
            id: 1,
            instances: instances_1,
            canonical_index: Some(0),
            clone_type: CloneType::Type1,
            avg_similarity: 1.0,
        },
        CloneGroup {
            id: 2,
            instances: instances_2,
            canonical_index: Some(0),
            clone_type: CloneType::Type2,
            avg_similarity: 0.9,
        },
    ];

    let summary = CloneSummary::from_groups(&groups);
    assert_eq!(summary.total_groups, 2);
    assert_eq!(summary.total_instances, 3);
    assert_eq!(summary.type1_count, 1);
    assert_eq!(summary.type2_count, 1);
    assert_eq!(summary.type3_count, 0);
    assert_eq!(summary.files_with_clones, 2);
}

// ============================================================================
// CloneConfig Tests
// ============================================================================

#[test]
fn test_clone_config_default() {
    let config = CloneConfig::default();
    assert!(config.min_similarity > 0.0);
    assert!(config.min_lines > 0);
}

#[test]
fn test_clone_config_rejects_invalid_similarity() {
    let nan_config = CloneConfig {
        min_similarity: f64::NAN,
        ..CloneConfig::default()
    };
    assert!(nan_config.validate().is_err());
    let high_config = CloneConfig {
        min_similarity: 1.1,
        ..CloneConfig::default()
    };
    assert!(high_config.validate().is_err());
}

#[test]
fn test_clone_config_builder() {
    let config = CloneConfig::default()
        .with_min_similarity(0.9)
        .with_auto_fix_threshold(95);

    assert!((config.min_similarity - 0.9).abs() < 0.001);
    assert_eq!(config.auto_fix_threshold, 95);
}

#[test]
fn test_clone_config_with_cfg_validation() {
    let config = CloneConfig::default().with_cfg_validation(true);
    assert!(config.cfg_validation);

    let config_disabled = CloneConfig::default().with_cfg_validation(false);
    assert!(!config_disabled.cfg_validation);

    // Default should be false
    let default_config = CloneConfig::default();
    assert!(!default_config.cfg_validation);
}

// ============================================================================
// CloneFinding Tests
// ============================================================================

#[test]
fn test_clone_finding_from_pair_canonical() {
    let instance_a = CloneInstance {
        file: PathBuf::from("test.py"),
        start_line: 10,
        end_line: 20,
        start_byte: 100,
        end_byte: 300,
        normalized_hash: 123,
        name: Some("canonical_func".to_owned()),
        node_kind: NodeKind::Function,
    };

    let instance_b = CloneInstance {
        file: PathBuf::from("test.py"),
        start_line: 30,
        end_line: 40,
        start_byte: 400,
        end_byte: 600,
        normalized_hash: 456,
        name: Some("duplicate_func".to_owned()),
        node_kind: NodeKind::Function,
    };

    let pair = ClonePair {
        instance_a,
        instance_b,
        similarity: 0.95,
        clone_type: CloneType::Type2,
        edit_distance: 2,
    };

    let finding = CloneFinding::from_pair(&pair, false, 90);
    assert_eq!(finding.rule_id, "CSP-C200");
    assert!(!finding.is_duplicate);
    assert_eq!(finding.fix_confidence, 90);
    assert_eq!(finding.name, Some("canonical_func".to_owned()));
}

#[test]
fn test_clone_finding_from_pair_duplicate() {
    let instance_a = CloneInstance {
        file: PathBuf::from("test.py"),
        start_line: 10,
        end_line: 20,
        start_byte: 100,
        end_byte: 300,
        normalized_hash: 123,
        name: Some("canonical_func".to_owned()),
        node_kind: NodeKind::Function,
    };

    let instance_b = CloneInstance {
        file: PathBuf::from("test.py"),
        start_line: 30,
        end_line: 40,
        start_byte: 400,
        end_byte: 600,
        normalized_hash: 456,
        name: Some("duplicate_func".to_owned()),
        node_kind: NodeKind::Function,
    };

    let pair = ClonePair {
        instance_a,
        instance_b,
        similarity: 0.95,
        clone_type: CloneType::Type1,
        edit_distance: 0,
    };

    let finding = CloneFinding::from_pair(&pair, true, 95);
    assert_eq!(finding.rule_id, "CSP-C100");
    assert!(finding.is_duplicate);
    assert_eq!(finding.severity, "WARNING");
    assert_eq!(finding.name, Some("duplicate_func".to_owned()));
    assert!(finding.message.contains("Duplicate of"));
}

// ============================================================================
// CloneRelation Tests
// ============================================================================

#[test]
fn test_clone_relation() {
    let relation = CloneRelation {
        file: PathBuf::from("related.py"),
        line: 25,
        end_line: 35,
        name: Some("related_func".to_owned()),
    };

    assert_eq!(relation.file, PathBuf::from("related.py"));
    assert_eq!(relation.line, 25);
    assert_eq!(relation.end_line, 35);
}

// ============================================================================
// FixContext and ConfidenceScorer Tests
// ============================================================================

#[test]
fn test_fix_context_cfg_validated() {
    let scorer = ConfidenceScorer::default();

    // Create a pair for testing with moderate similarity
    // (not too high, so CFG boost doesn't hit the 100 cap)
    let instance_a = CloneInstance {
        file: PathBuf::from("test.py"),
        start_line: 10,
        end_line: 20,
        start_byte: 100,
        end_byte: 300,
        normalized_hash: 123,
        name: Some("func_a".to_owned()),
        node_kind: NodeKind::Function,
    };

    let instance_b = CloneInstance {
        file: PathBuf::from("other.py"), // Different file = no same_file bonus
        start_line: 30,
        end_line: 40,
        start_byte: 400,
        end_byte: 600,
        normalized_hash: 456,
        name: Some("func_b".to_owned()),
        node_kind: NodeKind::Function,
    };

    let pair = ClonePair {
        instance_a,
        instance_b,
        similarity: 0.82,             // Moderate similarity (+10 boost, not +30)
        clone_type: CloneType::Type3, // Type3 gives -10
        edit_distance: 5,             // No edit distance bonus
    };

    // Without CFG validation
    let context_no_cfg = FixContext {
        cfg_validated: false,
        ..Default::default()
    };
    let score_no_cfg = scorer.score(&pair, &context_no_cfg);

    // With CFG validation (+15 boost)
    let context_with_cfg = FixContext {
        cfg_validated: true,
        ..Default::default()
    };
    let score_with_cfg = scorer.score(&pair, &context_with_cfg);

    // CFG validation should boost confidence by 15
    // Base: 50, similarity: +10, type3: -10, edit_distance: 0 = 50 without CFG
    // With CFG: 50 + 15 = 65
    assert!(score_with_cfg.score > score_no_cfg.score);
    assert_eq!(score_with_cfg.score - score_no_cfg.score, 15);
}

// ============================================================================
// Parser Tests
// ============================================================================

#[test]
fn test_parser_extract_function() {
    let source = "
def my_func(a, b):
    x = a + b
    y = x * 2
    z = y - a
    return z
";
    let path = PathBuf::from("test.py");
    let subtrees = extract_subtrees(source, &path).expect("Failed to parse source");

    assert_eq!(subtrees.len(), 1);
    let tree = &subtrees[0];
    assert_eq!(tree.name.as_deref(), Some("my_func"));
    assert_eq!(tree.node_type, SubtreeType::Function);
    assert_eq!(tree.start_line, 2);
}
