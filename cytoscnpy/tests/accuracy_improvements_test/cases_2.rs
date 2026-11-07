use super::*;

/// Test that regular unused imports are still flagged correctly.
#[test]
fn test_regular_imports_still_flagged() {
    let dir = project_tempdir();
    let file_path = dir.path().join("imports.py");
    let mut file = File::create(&file_path).unwrap();

    writeln!(
        file,
        r#"
import os  # Unused
import sys  # Used

print(sys.version)
"#
    )
    .unwrap();

    let mut analyzer = CytoScnPy::default().with_confidence(60).with_tests(false);
    let result = analyzer.analyze(dir.path());

    let unused_import_names: Vec<String> = result
        .unused_imports
        .iter()
        .map(|d| d.simple_name.clone())
        .collect();

    // os should be flagged as unused
    assert!(
        unused_import_names.contains(&"os".to_owned()),
        "os is unused and should be flagged"
    );

    // sys should NOT be flagged (it's used)
    assert!(
        !unused_import_names.contains(&"sys".to_owned()),
        "sys is used and should not be flagged"
    );
}
