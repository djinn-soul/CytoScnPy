//! Every unified pass can consume the snapshot after its file is removed.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use super::*;

fn reports(path: &Path, root: &Path, cache: &SourceCache) -> serde_json::Value {
    let files = vec![path.to_path_buf()];
    let sources = Some(cache);
    serde_json::json!({
        "architecture": crate::architecture::builder::build_architecture_with_sources(&files, &[root.to_path_buf()], sources),
        "context": crate::context::analyze_context_with_sources(&files, &crate::context::ContextConfig { no_git: true, ..Default::default() }, sources),
        "globals": crate::globals::scanner::scan_files_with_sources(&files, root, sources),
        "exceptions": crate::exceptions::scanner::scan_files_with_sources(&files, sources),
        "wildcards": crate::wildcards::scanner::scan_files_with_sources(&files, sources),
        "side_effects": crate::side_effects::scanner::scan_files_with_sources(&files, sources),
        "singletons": crate::singletons::scanner::scan_files_with_sources(&files, sources),
        "anti_patterns": crate::anti_patterns::scanner::scan_files_with_sources(&files, sources),
        "todos": crate::todos::scanner::scan_files_with_sources(&files, sources),
        "functions": crate::functions::scanner::scan_files_with_sources(&files, sources),
        "unreferenced": crate::unreferenced::analyzer::analyze_unreferenced_with_sources(&files, &crate::unreferenced::UnreferencedOptions::default(), sources),
        "duplicates": crate::duplicates::analyzer::analyze_duplicates_with_sources(&files, &crate::duplicates::DuplicatesOptions::default(), sources),
    })
}

#[test]
fn shared_passes_neither_reopen_nor_reparse_source_files() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("worker.py");
    std::fs::write(&path, "from os import *\nSTATE = []\n# TODO: cleanup\neval(input())\ndef worker(x):\n    if x > 500:\n        return x\n    return 0\ntry:\n    pass\nexcept:\n    pass\n").unwrap();
    let snapshot = load_source(&path, None).unwrap().into_owned();
    let cache = [(path.clone(), Ok(snapshot))].into_iter().collect();
    let expected = reports(&path, temp.path(), &cache);
    std::fs::remove_file(&path).unwrap();
    assert_eq!(reports(&path, temp.path(), &cache), expected);
    assert_eq!(expected["functions"]["files_scanned"], 1);
    assert!(!expected["wildcards"]["matches"]
        .as_array()
        .unwrap()
        .is_empty());
}
