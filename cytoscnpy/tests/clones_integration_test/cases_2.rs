use super::*;

#[test]
fn test_parse_error_handling() {
    let source = "this is not python code !!! invalid syntax";
    let path = PathBuf::from("invalid.py");

    let files = vec![(path, source.to_string())];
    let detector = CloneDetector::new();

    // Should not panic, should skip invalid files
    let result = detector.detect(&files);
    assert_eq!(result.pairs.len(), 0);
}

#[test]
fn test_subtree_node_size() {
    let node = SubtreeNode {
        kind: "root".into(),
        label: None,
        children: vec![
            SubtreeNode {
                kind: "child1".into(),
                label: None,
                children: vec![],
            },
            cytoscnpy::clones::SubtreeNode {
                kind: "child2".into(),
                label: None,
                children: vec![],
            },
        ],
    };

    assert_eq!(node.size(), 3);
}
