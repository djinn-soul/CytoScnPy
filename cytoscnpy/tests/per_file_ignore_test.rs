//! Tests for per-file ignore rule behavior.
#![allow(clippy::unwrap_used)]

use cytoscnpy::analyzer::CytoScnPy;
use cytoscnpy::config::Config;
use rustc_hash::FxHashMap;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

#[test]
fn per_file_ignore_rules_apply() {
    let mut mapping = FxHashMap::default();
    mapping.insert("tests/*".to_owned(), vec!["S101".to_owned()]);
    mapping.insert("src/__init__.py".to_owned(), vec!["F401".to_owned()]);

    let mut config = Config::default();
    config.cytoscnpy.per_file_ignores = Some(mapping);

    let analyzer = CytoScnPy::new(
        60,
        false,
        false,
        false,
        false,
        Vec::new(),
        Vec::new(),
        false,
        false,
        config,
    )
    .with_root(PathBuf::from("project"));

    assert!(analyzer.is_rule_ignored_for_path(Path::new("project/tests/test_case.py"), "s101",));
    assert!(!analyzer.is_rule_ignored_for_path(Path::new("project/tests/test_case.py"), "E501",));
    assert!(analyzer.is_rule_ignored_for_path(Path::new("project/src/__init__.py"), "F401",));
    assert!(!analyzer.is_rule_ignored_for_path(Path::new("project/src/__init__.py"), "S101",));
}

#[test]
fn per_file_ignore_suppresses_min_mi_finding() {
    let temp = tempdir().unwrap();
    let root = temp.path().to_path_buf();
    let src_dir = root.join("src");
    std::fs::create_dir_all(&src_dir).unwrap();
    let file_path = src_dir.join("main.py");
    std::fs::write(&file_path, "def f():\n    return 1\n").unwrap();

    let mut mapping = FxHashMap::default();
    mapping.insert("src/main.py".to_owned(), vec!["CSP-Q303".to_owned()]);

    let mut config = Config::default();
    config.cytoscnpy.min_mi = Some(100.0);
    config.cytoscnpy.per_file_ignores = Some(mapping);

    let mut analyzer = CytoScnPy::new(
        60,
        false,
        false,
        false,
        false,
        Vec::new(),
        Vec::new(),
        false,
        false,
        config,
    )
    .with_root(root.clone());

    let result = analyzer.analyze(&root);
    assert!(
        !result
            .quality
            .iter()
            .any(|f| f.rule_id == "CSP-Q303" && f.file == file_path),
        "CSP-Q303 should respect per-file ignore for src/main.py"
    );
}

#[test]
fn per_file_ignore_suppresses_taint_finding() {
    let temp = tempdir().unwrap();
    let root = temp.path().to_path_buf();
    let src_dir = root.join("src");
    std::fs::create_dir_all(&src_dir).unwrap();
    let file_path = src_dir.join("main.py");
    std::fs::write(&file_path, "import os\n\ncmd = input()\nos.system(cmd)\n").unwrap();

    let mut mapping = FxHashMap::default();
    mapping.insert("src/main.py".to_owned(), vec!["CSP-D003".to_owned()]);

    let mut config = Config::default();
    config.cytoscnpy.per_file_ignores = Some(mapping);
    config.cytoscnpy.danger_config.enable_taint = Some(true);

    let mut analyzer = CytoScnPy::new(
        60,
        false,
        true,
        false,
        false,
        Vec::new(),
        Vec::new(),
        false,
        false,
        config,
    )
    .with_root(root.clone());

    let result = analyzer.analyze(&root);
    assert!(
        !result
            .taint_findings
            .iter()
            .any(|f| f.rule_id == "CSP-D003" && f.file == file_path),
        "CSP-D003 taint finding should respect per-file ignore for src/main.py"
    );
}
