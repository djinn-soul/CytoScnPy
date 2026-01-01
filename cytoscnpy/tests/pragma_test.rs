//! Tests for pragma/inline-ignore functionality.
#![allow(clippy::unwrap_used)]

use cytoscnpy::analyzer::CytoScnPy;
use std::fs::File;
use std::io::Write;
use tempfile::TempDir;

fn project_tempdir() -> TempDir {
    let mut target_dir = std::env::current_dir().unwrap();
    target_dir.push("target");
    target_dir.push("test-pragma-tmp");
    std::fs::create_dir_all(&target_dir).unwrap();
    tempfile::Builder::new()
        .prefix("pragma_test_")
        .tempdir_in(target_dir)
        .unwrap()
}

#[test]
fn test_pragma_no_cytoscnpy() {
    let dir = project_tempdir();
    let file_path = dir.path().join("main.py");
    let mut file = File::create(&file_path).unwrap();

    writeln!(
        file,
        r"
def unused_no_ignore():
    pass

def unused_ignore(): # pragma: no cytoscnpy
    pass

def used():
    pass

used()
"
    )
    .unwrap();

    let mut analyzer = CytoScnPy::default().with_confidence(60).with_tests(false);
    let result = analyzer.analyze(dir.path());

    let unreachable: Vec<String> = result
        .unused_functions
        .iter()
        .map(|f| f.simple_name.clone())
        .collect();

    assert!(unreachable.contains(&"unused_no_ignore".to_owned()));
    assert!(!unreachable.contains(&"unused_ignore".to_owned()));
    assert!(!unreachable.contains(&"used".to_owned()));
}
