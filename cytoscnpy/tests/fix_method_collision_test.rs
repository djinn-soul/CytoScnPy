//! Integration test for method-fix targeting when method names collide across classes.
#![allow(clippy::unwrap_used)]

use cytoscnpy::entry_point::run_with_args_to;
use tempfile::tempdir;

#[test]
fn fix_removes_only_target_method_when_names_collide_across_classes() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("sample.py");
    let source = r#"
class A:
    def ping(self):
        return "a"

class B:
    def ping(self):  # pragma: no cytoscnpy
        return "b"

a = A()
b = B()
"#;
    std::fs::write(&file_path, source).unwrap();

    let mut buffer = Vec::new();
    let result = run_with_args_to(
        vec![
            "--fix".to_owned(),
            "--apply".to_owned(),
            "--confidence".to_owned(),
            "80".to_owned(),
            file_path.to_string_lossy().to_string(),
        ],
        &mut buffer,
    )
    .unwrap();
    assert_eq!(result, 0);

    let updated = std::fs::read_to_string(&file_path).unwrap();
    assert_eq!(updated.matches("def ping").count(), 1);
    assert!(updated.contains("class B:\n    def ping"));
    assert!(!updated.contains("class A:\n    def ping"));
    assert!(updated.contains("class A:\n    pass"));
}
