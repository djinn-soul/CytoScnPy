//! Regression test for instance-method call reference capture.

#![allow(clippy::unwrap_used)]

use cytoscnpy::analyzer::CytoScnPy;
use std::fs;
use tempfile::tempdir;

#[test]
fn captures_attribute_call_reference_for_instance_method() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("class_methods.py");
    fs::write(
        &file_path,
        r#"
class UsedClass:
    def used_method(self):
        return "used"

obj = UsedClass()
obj.used_method()
"#,
    )
    .unwrap();

    let analyzer = CytoScnPy::default().with_confidence(60);
    let file_result = analyzer.process_single_file(&file_path, dir.path());

    assert!(
        file_result
            .references
            .get(".used_method")
            .copied()
            .unwrap_or(0)
            > 0,
        "expected `.used_method` attribute reference to be captured, got refs: {:?}",
        file_result.references
    );
}
