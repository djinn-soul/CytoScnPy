use super::*;

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
