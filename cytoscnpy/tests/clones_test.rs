use cytoscnpy::clones::{extract_subtrees, CloneDetector, SubtreeType};
use std::path::PathBuf;

#[test]
fn test_parser_extract_function() {
    let source = "
def my_func(a, b):
    return a + b
";
    let path = PathBuf::from("test.py");
    let subtrees = extract_subtrees(source, &path).unwrap();

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
    let subtrees = extract_subtrees(source, &path).unwrap();

    // Should find the class and the method inside it
    assert_eq!(subtrees.len(), 2);

    let class_tree = subtrees
        .iter()
        .find(|s| s.node_type == SubtreeType::Class)
        .unwrap();
    assert_eq!(class_tree.name.as_deref(), Some("MyClass"));

    let method_tree = subtrees
        .iter()
        .find(|s| s.node_type == SubtreeType::Method)
        .unwrap();
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
        (PathBuf::from("file1.py"), source1.to_string()),
        (PathBuf::from("file2.py"), source2.to_string()),
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
        (PathBuf::from("file1.py"), source1.to_string()),
        (PathBuf::from("file2.py"), source2.to_string()),
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
        (PathBuf::from("file1.py"), source1.to_string()),
        (PathBuf::from("file2.py"), source2.to_string()),
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
    let subtrees = extract_subtrees(source, &path).unwrap();
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
    let subtrees = extract_subtrees(source, &path).unwrap();

    assert_eq!(subtrees.len(), 1);
    let tree = &subtrees[0];

    // We expect a good number of nodes.
    // To verify coverage, we mainly rely on execution, but let's check basic structure.
    assert_eq!(tree.node_type, SubtreeType::Function);

    // Convert to instance to trigger hashing logic for all these nodes
    let instance = tree.to_instance();
    assert!(instance.normalized_hash != 0);
}
