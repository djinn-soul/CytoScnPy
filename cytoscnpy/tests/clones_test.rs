//! Tests for the clone detection module.
//!
//! This module contains unit tests for clone type classification, data structures,
//! serialization, parser functionality, and clone detection algorithms.

#![allow(clippy::expect_used)]

use cytoscnpy::clones::{
    extract_subtrees, CloneConfig, CloneDetector, CloneFinding, CloneGroup, CloneInstance,
    ClonePair, CloneRelation, CloneSummary, CloneType, ConfidenceScorer, FixContext, NodeKind,
    SubtreeType,
};
use std::path::PathBuf;

// ============================================================================
// CloneType Tests
// ============================================================================

#[test]
fn test_clone_type_confidence_bonus() {
    assert_eq!(CloneType::Type1.confidence_bonus(), 25);
    assert_eq!(CloneType::Type2.confidence_bonus(), 15);
    assert_eq!(CloneType::Type3.confidence_bonus(), -10);
}

#[test]
fn test_clone_type_serialization() -> Result<(), Box<dyn std::error::Error>> {
    let type1 = CloneType::Type1;
    let type2 = CloneType::Type2;
    let type3 = CloneType::Type3;

    // Test that they can be serialized
    let json1 = serde_json::to_string(&type1)?;
    let json2 = serde_json::to_string(&type2)?;
    let json3 = serde_json::to_string(&type3)?;

    assert!(json1.contains("Type1"));
    assert!(json2.contains("Type2"));
    assert!(json3.contains("Type3"));

    // Test deserialization
    let parsed: CloneType = serde_json::from_str(&json1)?;
    assert_eq!(parsed, CloneType::Type1);

    Ok(())
}

// ============================================================================
// CloneInstance Tests
// ============================================================================

#[test]
fn test_clone_instance_creation() {
    let instance = CloneInstance {
        file: PathBuf::from("test.py"),
        start_line: 10,
        end_line: 20,
        start_byte: 100,
        end_byte: 300,
        normalized_hash: 12345,
        name: Some("my_function".to_owned()),
        node_kind: NodeKind::Function,
    };

    assert_eq!(instance.file, PathBuf::from("test.py"));
    assert_eq!(instance.start_line, 10);
    assert_eq!(instance.end_line, 20);
    assert_eq!(instance.name, Some("my_function".to_owned()));
}

#[test]
fn test_clone_instance_serialization() -> Result<(), Box<dyn std::error::Error>> {
    let instance = CloneInstance {
        file: PathBuf::from("test.py"),
        start_line: 10,
        end_line: 20,
        start_byte: 100,
        end_byte: 300,
        normalized_hash: 12345,
        name: Some("my_function".to_owned()),
        node_kind: NodeKind::Function,
    };

    let json = serde_json::to_string(&instance)?;
    assert!(json.contains("test.py"));
    assert!(json.contains("my_function"));

    let parsed: CloneInstance = serde_json::from_str(&json)?;
    assert_eq!(parsed.start_line, 10);
    assert_eq!(parsed.name, Some("my_function".to_owned()));

    Ok(())
}

// ============================================================================
// ClonePair Tests
// ============================================================================

#[test]
fn test_clone_pair_same_file() {
    let instance_a = CloneInstance {
        file: PathBuf::from("utils.py"),
        start_line: 10,
        end_line: 20,
        start_byte: 100,
        end_byte: 300,
        normalized_hash: 123,
        name: Some("func_a".to_owned()),
        node_kind: NodeKind::Function,
    };

    let instance_b = CloneInstance {
        file: PathBuf::from("utils.py"),
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
        similarity: 0.95,
        clone_type: CloneType::Type2,
        edit_distance: 2,
    };

    assert!(pair.is_same_file());
    assert!((pair.similarity - 0.95).abs() < f64::EPSILON);
}

#[test]
fn test_clone_pair_different_files() {
    let instance_a = CloneInstance {
        file: PathBuf::from("file_a.py"),
        start_line: 10,
        end_line: 20,
        start_byte: 100,
        end_byte: 300,
        normalized_hash: 123,
        name: Some("func_a".to_owned()),
        node_kind: NodeKind::Function,
    };

    let instance_b = CloneInstance {
        file: PathBuf::from("file_b.py"),
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
        similarity: 0.85,
        clone_type: CloneType::Type3,
        edit_distance: 5,
    };

    assert!(!pair.is_same_file());
}

#[test]
fn test_clone_pair_canonical() {
    let instance_a = CloneInstance {
        file: PathBuf::from("test.py"),
        start_line: 50,
        end_line: 60,
        start_byte: 500,
        end_byte: 700,
        normalized_hash: 123,
        name: Some("later_func".to_owned()),
        node_kind: NodeKind::Function,
    };

    let instance_b = CloneInstance {
        file: PathBuf::from("test.py"),
        start_line: 10,
        end_line: 20,
        start_byte: 100,
        end_byte: 300,
        normalized_hash: 456,
        name: Some("earlier_func".to_owned()),
        node_kind: NodeKind::Function,
    };

    let pair = ClonePair {
        instance_a,
        instance_b,
        similarity: 0.90,
        clone_type: CloneType::Type1,
        edit_distance: 0,
    };

    // Canonical should be the one with smaller start_byte
    let canonical = pair.canonical();
    assert_eq!(canonical.name, Some("earlier_func".to_owned()));
}

// ============================================================================
// CloneGroup Tests
// ============================================================================

#[test]
fn test_clone_group_creation() {
    let instances = vec![
        CloneInstance {
            file: PathBuf::from("test.py"),
            start_line: 10,
            end_line: 20,
            start_byte: 100,
            end_byte: 300,
            normalized_hash: 123,
            name: Some("func_1".to_owned()),
            node_kind: NodeKind::Function,
        },
        CloneInstance {
            file: PathBuf::from("test.py"),
            start_line: 30,
            end_line: 40,
            start_byte: 400,
            end_byte: 600,
            normalized_hash: 456,
            name: Some("func_2".to_owned()),
            node_kind: NodeKind::Function,
        },
    ];

    let group = CloneGroup {
        id: 1,
        instances,
        canonical_index: Some(0),
        clone_type: CloneType::Type2,
        avg_similarity: 0.92,
    };

    assert_eq!(group.id, 1);
    assert_eq!(group.instances.len(), 2);
    assert!(group.canonical().is_some());
    assert_eq!(group.duplicates().len(), 1);
}

// ============================================================================
// CloneSummary Tests
// ============================================================================

#[test]
fn test_clone_summary_empty() {
    let groups: Vec<CloneGroup> = vec![];
    let summary = CloneSummary::from_groups(&groups);

    assert_eq!(summary.total_groups, 0);
    assert_eq!(summary.total_instances, 0);
    assert_eq!(summary.type1_count, 0);
    assert_eq!(summary.type2_count, 0);
    assert_eq!(summary.type3_count, 0);
}

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
    return a + b
";
    let path = PathBuf::from("test.py");
    let subtrees = extract_subtrees(source, &path).expect("Failed to parse source");

    assert_eq!(subtrees.len(), 1);
    let tree = &subtrees[0];
    assert_eq!(tree.name.as_deref(), Some("my_func"));
    assert_eq!(tree.node_type, SubtreeType::Function);
    assert_eq!(tree.start_line, 2);
}

#[test]
fn test_parser_extract_class() {
    let source = "
class MyClass:
    def method(self):
        pass
";
    let path = PathBuf::from("test.py");
    let subtrees = extract_subtrees(source, &path).expect("Failed to parse source");

    // Should find the class and the method inside it
    assert_eq!(subtrees.len(), 2);

    let class_tree = subtrees
        .iter()
        .find(|s| s.node_type == SubtreeType::Class)
        .expect("Class subtree not found");
    assert_eq!(class_tree.name.as_deref(), Some("MyClass"));

    let method_tree = subtrees
        .iter()
        .find(|s| s.node_type == SubtreeType::Method)
        .expect("Method subtree not found");
    // Method name might be just "method" or fully qualified depending on implementation,
    // strictly parser.rs sets it to function name "method"
    assert_eq!(method_tree.name.as_deref(), Some("method"));
}

#[test]
fn test_clone_detection_type1_exact() {
    let source1 = "
def add(x, y):
    return x + y
";
    let source2 = "
def add(x, y):
    return x + y
";

    let detector = CloneDetector::new();
    let files = vec![
        (PathBuf::from("file1.py"), source1.to_owned()),
        (PathBuf::from("file2.py"), source2.to_owned()),
    ];

    let result = detector.detect(&files);

    // Should find 1 pair
    assert!(!result.pairs.is_empty(), "Should detect exact clone");
    let pair = &result.pairs[0];
    assert!(pair.similarity > 0.99);
}

#[test]
fn test_clone_detection_type2_renamed() {
    let source1 = "
def calculate(a, b):
    sum_val = a + b
    return sum_val * 2
";
    let source2 = "
def compute(x, y):
    total = x + y
    return total * 2
";

    let detector = CloneDetector::new();
    let files = vec![
        (PathBuf::from("file1.py"), source1.to_owned()),
        (PathBuf::from("file2.py"), source2.to_owned()),
    ];

    let result = detector.detect(&files);

    assert!(!result.pairs.is_empty(), "Should detect renamed clone");
    // Verify it's type 2 or high similarity
    let pair = &result.pairs[0];
    assert!(
        pair.similarity > 0.8,
        "Similarity should be high for renamed code"
    );
}

#[test]
fn test_no_false_positives() {
    let source1 = "
def func_a(x):
    if x:
        print('hello')
";
    let source2 = "
def func_b(y):
    while y:
        y -= 1
        return y
";

    let detector = CloneDetector::new();
    let files = vec![
        (PathBuf::from("file1.py"), source1.to_owned()),
        (PathBuf::from("file2.py"), source2.to_owned()),
    ];

    let result = detector.detect(&files);

    assert!(
        result.pairs.is_empty(),
        "Should not detect clones between different logic"
    );
}

#[test]
fn test_subtree_hashing() {
    let source = "
def func():
    a = 1
    b = 2
    return a + b
";
    let path = PathBuf::from("test.py");
    let subtrees = extract_subtrees(source, &path).expect("Failed to parse source");
    let instance = subtrees[0].to_instance();

    assert!(instance.normalized_hash != 0, "Hash should be computed");
}

#[test]
fn test_parser_complex_structures() {
    let source = "
def complex(a):
    if a > 0:
        return 1
    elif a < 0:
        return -1
    else:
        for i in range(10):
            while True:
                try:
                    print(i)
                except Exception:
                    break
    return 0
    ";
    let path = PathBuf::from("test.py");
    let subtrees = extract_subtrees(source, &path).expect("Failed to parse source");

    assert_eq!(subtrees.len(), 1);
    let tree = &subtrees[0];

    // We expect a good number of nodes.
    // To verify coverage, we mainly rely on execution, but let's check basic structure.
    assert_eq!(tree.node_type, SubtreeType::Function);

    // Convert to instance to trigger hashing logic for all these nodes
    let instance = tree.to_instance();
    assert!(instance.normalized_hash != 0);
}
