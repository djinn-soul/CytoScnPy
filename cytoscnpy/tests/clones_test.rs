//! Tests for the clones module (Clone Detection).

use cytoscnpy::clones::{
    CloneConfig, CloneFinding, CloneGroup, CloneInstance, ClonePair, CloneRelation, CloneSummary,
    CloneType,
};
use std::path::PathBuf;

#[test]
fn test_clone_type_confidence_bonus() {
    assert_eq!(CloneType::Type1.confidence_bonus(), 25);
    assert_eq!(CloneType::Type2.confidence_bonus(), 15);
    assert_eq!(CloneType::Type3.confidence_bonus(), -10);
}

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
    };

    assert_eq!(instance.file, PathBuf::from("test.py"));
    assert_eq!(instance.start_line, 10);
    assert_eq!(instance.end_line, 20);
    assert_eq!(instance.name, Some("my_function".to_owned()));
}

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
    };

    let instance_b = CloneInstance {
        file: PathBuf::from("utils.py"),
        start_line: 30,
        end_line: 40,
        start_byte: 400,
        end_byte: 600,
        normalized_hash: 456,
        name: Some("func_b".to_owned()),
    };

    let pair = ClonePair {
        instance_a,
        instance_b,
        similarity: 0.95,
        clone_type: CloneType::Type2,
        edit_distance: 2,
    };

    assert!(pair.is_same_file());
    assert_eq!(pair.similarity, 0.95);
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
    };

    let instance_b = CloneInstance {
        file: PathBuf::from("file_b.py"),
        start_line: 30,
        end_line: 40,
        start_byte: 400,
        end_byte: 600,
        normalized_hash: 456,
        name: Some("func_b".to_owned()),
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
    };

    let instance_b = CloneInstance {
        file: PathBuf::from("test.py"),
        start_line: 10,
        end_line: 20,
        start_byte: 100,
        end_byte: 300,
        normalized_hash: 456,
        name: Some("earlier_func".to_owned()),
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
        },
        CloneInstance {
            file: PathBuf::from("test.py"),
            start_line: 30,
            end_line: 40,
            start_byte: 400,
            end_byte: 600,
            normalized_hash: 456,
            name: Some("func_2".to_owned()),
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
        },
        CloneInstance {
            file: PathBuf::from("a.py"),
            start_line: 20,
            end_line: 30,
            start_byte: 200,
            end_byte: 300,
            normalized_hash: 2,
            name: None,
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
fn test_clone_finding_from_pair_canonical() {
    let instance_a = CloneInstance {
        file: PathBuf::from("test.py"),
        start_line: 10,
        end_line: 20,
        start_byte: 100,
        end_byte: 300,
        normalized_hash: 123,
        name: Some("canonical_func".to_owned()),
    };

    let instance_b = CloneInstance {
        file: PathBuf::from("test.py"),
        start_line: 30,
        end_line: 40,
        start_byte: 400,
        end_byte: 600,
        normalized_hash: 456,
        name: Some("duplicate_func".to_owned()),
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
    };

    let instance_b = CloneInstance {
        file: PathBuf::from("test.py"),
        start_line: 30,
        end_line: 40,
        start_byte: 400,
        end_byte: 600,
        normalized_hash: 456,
        name: Some("duplicate_func".to_owned()),
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
    assert!(finding.message.contains("Duplicate code"));
}

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

#[test]
fn test_clone_type_serialization() {
    let type1 = CloneType::Type1;
    let type2 = CloneType::Type2;
    let type3 = CloneType::Type3;

    // Test that they can be serialized
    let json1 = serde_json::to_string(&type1).unwrap();
    let json2 = serde_json::to_string(&type2).unwrap();
    let json3 = serde_json::to_string(&type3).unwrap();

    assert!(json1.contains("Type1"));
    assert!(json2.contains("Type2"));
    assert!(json3.contains("Type3"));

    // Test deserialization
    let parsed: CloneType = serde_json::from_str(&json1).unwrap();
    assert_eq!(parsed, CloneType::Type1);
}

#[test]
fn test_clone_instance_serialization() {
    let instance = CloneInstance {
        file: PathBuf::from("test.py"),
        start_line: 10,
        end_line: 20,
        start_byte: 100,
        end_byte: 300,
        normalized_hash: 12345,
        name: Some("my_function".to_owned()),
    };

    let json = serde_json::to_string(&instance).unwrap();
    assert!(json.contains("test.py"));
    assert!(json.contains("my_function"));

    let parsed: CloneInstance = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.start_line, 10);
    assert_eq!(parsed.name, Some("my_function".to_owned()));
}
