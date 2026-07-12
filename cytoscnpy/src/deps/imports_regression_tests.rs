use super::{extract_import_scan, extract_imports};
use std::fs;
use tempfile::tempdir;

fn imports_from(source: &str) -> anyhow::Result<rustc_hash::FxHashSet<String>> {
    let dir = tempdir()?;
    fs::write(dir.path().join("module.py"), source)?;
    Ok(extract_imports(&[dir.path().to_path_buf()], &[], false))
}

#[test]
fn skips_elif_type_checking_imports_only_when_typing_provides_the_name() -> anyhow::Result<()> {
    let imports = imports_from(
        "from typing import TYPE_CHECKING\nif False:\n    pass\nelif TYPE_CHECKING:\n    import pydantic\n",
    )?;

    assert!(!imports.contains("pydantic"));
    let imports_without_typing = imports_from("if TYPE_CHECKING:\n    import rich\n")?;
    assert!(imports_without_typing.contains("rich"));
    Ok(())
}

#[test]
fn finds_dynamic_imports_in_all_direct_expression_positions() -> anyhow::Result<()> {
    let imports = imports_from(
        "import importlib\ndef decorator(value): return value\n@decorator(importlib.import_module('decorated'))\ndef function(default=importlib.import_module('default'), arg: importlib.import_module('annotation') = None): pass\nclass Child(importlib.import_module('base')): pass\nif False: pass\nelif importlib.import_module('elif_test'): pass\nmatch importlib.import_module('subject'):\n    case _ if importlib.import_module('guard'): pass\n",
    )?;

    for name in [
        "decorated",
        "default",
        "annotation",
        "base",
        "elif_test",
        "subject",
        "guard",
    ] {
        assert!(
            imports.contains(name),
            "missing dynamic import {name}: {imports:?}"
        );
    }
    Ok(())
}

#[test]
fn handles_keyword_dynamic_imports_and_skips_relative_ones() -> anyhow::Result<()> {
    let imports = imports_from(
        "import importlib\nimportlib.import_module(name='keyword_dep')\nimportlib.import_module('.plugin', __package__)\n",
    )?;

    assert!(imports.contains("keyword_dep"));
    assert!(!imports.contains(""));
    Ok(())
}

#[test]
fn records_one_location_per_top_level_import_per_statement() -> anyhow::Result<()> {
    let dir = tempdir()?;
    fs::write(
        dir.path().join("module.py"),
        "import requests.api, requests.models\n",
    )?;
    let scan = extract_import_scan(&[dir.path().to_path_buf()], &[], false);

    assert_eq!(
        scan.occurrences
            .iter()
            .filter(|occurrence| occurrence.name == "requests")
            .count(),
        1
    );
    Ok(())
}

#[test]
fn dynamic_import_locations_use_character_columns() -> anyhow::Result<()> {
    let dir = tempdir()?;
    fs::write(
        dir.path().join("module.py"),
        "import importlib\né = importlib.import_module('requests')\n",
    )?;
    let scan = extract_import_scan(&[dir.path().to_path_buf()], &[], false);
    let occurrence = scan
        .occurrences
        .iter()
        .find(|occurrence| occurrence.name == "requests")
        .expect("dynamic import should be recorded");

    assert_eq!(occurrence.column, 5);
    Ok(())
}
