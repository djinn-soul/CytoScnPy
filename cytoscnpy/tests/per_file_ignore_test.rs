use cytoscnpy::analyzer::CytoScnPy;
use cytoscnpy::config::Config;
use rustc_hash::FxHashMap;
use std::path::{Path, PathBuf};

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
