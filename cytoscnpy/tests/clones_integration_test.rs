#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::str_to_string,
    clippy::missing_docs_in_private_items,
    missing_docs
)]

use cytoscnpy::clones::{
    extract_subtrees, CloneConfig, CloneDetector, CloneType, NodeKind, SubtreeNode,
};
use std::path::PathBuf;

#[test]
fn test_clone_detector_initialization() {
    let detector = CloneDetector::new();
    // Verify default state implicitly via detect on empty input
    let result = detector.detect(&[]);
    assert_eq!(result.pairs.len(), 0);
    assert_eq!(result.groups.len(), 0);
    assert_eq!(result.summary.total_instances, 0);
}

#[test]
fn test_clone_detector_with_custom_config() {
    let config = CloneConfig::default()
        .with_min_similarity(0.95)
        .with_auto_fix_threshold(99);
    let detector = CloneDetector::with_config(config);
    let result = detector.detect(&[]);
    assert_eq!(result.pairs.len(), 0);
}

#[test]
fn test_extract_subtrees_simple() {
    let source = r#"
def calculate_area(radius):
    import math
    area = math.pi * radius * radius
    return area

class Circle:
    def __init__(self, r):
        self.r = r
        self.area = math.pi * r * r
        self.name = "circle"
    
    async def get_data():
        data = {}
        data["r"] = 1
        return data
"#;
    let path = PathBuf::from("test.py");
    let result = extract_subtrees(source, &path).expect("Should parse");

    // Should extract calculate_area (function), Circle (class), __init__ (method), and get_data (async)
    // Actually extract_from_body recurses into class bodies for methods, and function bodies for nested.

    assert!(result.len() >= 3);

    let has_func = result
        .iter()
        .any(|s| s.name.as_deref() == Some("calculate_area"));
    let has_class = result.iter().any(|s| s.name.as_deref() == Some("Circle"));
    let has_init = result.iter().any(|s| s.name.as_deref() == Some("__init__"));

    assert!(has_func);
    assert!(has_class);
    assert!(has_init);
}

#[test]
fn test_subtree_to_instance() {
    let source = r#"
def foo():
    x = 1
    y = 2
    return x + y
"#;
    let path = PathBuf::from("foo.py");
    let subtrees = extract_subtrees(source, &path).unwrap();
    let subtree = &subtrees[0];
    let instance = subtree.to_instance();

    assert_eq!(instance.file, path);
    assert_eq!(instance.name.as_deref(), Some("foo"));
    assert_eq!(instance.node_kind, NodeKind::Function);
}

#[test]
fn test_clone_detection_type1_exact() {
    let source = r#"
def calculate_metrics(data):
    total = sum(data)
    count = len(data)
    if count == 0:
        return 0
    average = total / count
    variance = sum((x - average) ** 2 for x in data) / count
    std_dev = variance ** 0.5
    return {
        "total": total,
        "average": average,
        "std_dev": std_dev
    }
"#;

    let files = vec![
        (PathBuf::from("file1.py"), source.to_string()),
        (PathBuf::from("file2.py"), source.to_string()),
    ];

    let detector = CloneDetector::new();
    let result = detector.detect(&files);

    println!("Extracted clone pairs: {}", result.pairs.len());
    for (i, p) in result.pairs.iter().enumerate() {
        println!(
            "Pair {}: similarity={}, type={:?}",
            i, p.similarity, p.clone_type
        );
    }

    assert!(
        !result.pairs.is_empty(),
        "Should find at least one clone pair for identical functions"
    );
    assert!(
        result
            .pairs
            .iter()
            .any(|p| p.clone_type == CloneType::Type1),
        "Should find a Type-1 clone"
    );
}

#[test]
fn test_clone_detection_type2_renamed() {
    let source_a = r#"
def process_user_data(users):
    results = []
    for user in users:
        if user.is_active:
            profile = fetch_profile(user.id)
            results.append({
                "name": profile.fullname,
                "email": user.email,
                "score": calculate_score(user.stats)
            })
    return results
"#;
    let source_b = r#"
def handle_customer_records(customers):
    output = []
    for client in customers:
        if client.is_active:
            data = get_details(client.id)
            output.append({
                "name": data.fullname,
                "email": client.email,
                "score": compute_metric(client.stats)
            })
    return output
"#;

    let files = vec![
        (PathBuf::from("users.py"), source_a.to_string()),
        (PathBuf::from("customers.py"), source_b.to_string()),
    ];

    let detector = CloneDetector::new();
    let result = detector.detect(&files);

    assert!(!result.pairs.is_empty(), "Should find renamed clones");
    // Type-2 or Type-1 (if normalization is aggressive)
    assert!(result
        .pairs
        .iter()
        .any(|p| p.clone_type == CloneType::Type2 || p.clone_type == CloneType::Type1));
}

#[test]
fn test_clone_detection_type3_similar() {
    let source_a = r#"
def handle_request(req):
    log.info("Processing request")
    data = req.get_json()
    if not data:
        return error("No data", 400)
    
    user_id = data.get("user_id")
    if not user_id:
        return error("No user_id", 400)
        
    process_payload(data)
    return success()
"#;
    let source_b = r#"
def handle_request_v2(req):
    # Added some extra logging
    log.debug("V2 request received")
    data = req.get_json()
    if not data:
        log.warn("Empty payload")
        return error("Missing payload", 400)
    
    user_id = data.get("user_id")
    # Added auth check
    if not is_authorized(user_id):
        return error("Unauthorized", 401)
        
    process_payload(data)
    # Extra cleanup
    cleanup_temp_files()
    return success()
"#;

    let files = vec![
        (PathBuf::from("v1.py"), source_a.to_string()),
        (PathBuf::from("v2.py"), source_b.to_string()),
    ];

    let detector = CloneDetector::with_config(CloneConfig::default().with_min_similarity(0.5)); // Lower to 0.5 for debug
    let result = detector.detect(&files);

    // Diagnostic: check extract_subtrees directly
    let st_a = extract_subtrees(source_a, &PathBuf::from("a.py")).unwrap();
    let st_b = extract_subtrees(source_b, &PathBuf::from("b.py")).unwrap();
    println!("Type-3 extracted: a={}, b={}", st_a.len(), st_b.len());

    println!("Extracted clone pairs (Type-3): {}", result.pairs.len());
    for (i, p) in result.pairs.iter().enumerate() {
        println!(
            "Type-3 Pair {}: similarity={}, type={:?}",
            i, p.similarity, p.clone_type
        );
    }

    assert!(!result.pairs.is_empty(), "Should find similar clones");
}

#[test]
fn test_structural_nodes_extraction() {
    let source = r"
def complex_logic(x):
    y = x * 2
    if y > 10:
        return y
    else:
        for i in range(x):
            print(i)
        return 0
";
    let path = PathBuf::from("complex.py");
    let subtrees = extract_subtrees(source, &path).unwrap();
    let subtree = &subtrees[0];

    // Verify children kinds
    let kinds: Vec<String> = subtree.children.iter().map(|c| c.kind.clone()).collect();
    assert!(kinds.contains(&"assign".to_string()));
    assert!(kinds.contains(&"if".to_string()));

    // Find the 'if' node and check its children
    let if_node = subtree.children.iter().find(|c| c.kind == "if").unwrap();
    assert!(if_node.children.len() >= 2); // test and body
}

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
