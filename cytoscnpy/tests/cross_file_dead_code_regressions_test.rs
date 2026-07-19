//! Regression tests for cross-file dead-code name and import resolution.
#![allow(clippy::unwrap_used)]

use cytoscnpy::analyzer::{AnalysisResult, CytoScnPy};
use std::path::Path;
use tempfile::TempDir;

fn project_tempdir() -> TempDir {
    let target_dir = std::env::current_dir()
        .unwrap()
        .join("target")
        .join("test-cross-file-dead-code-tmp");
    std::fs::create_dir_all(&target_dir).unwrap();
    tempfile::Builder::new()
        .prefix("cross_file_dead_code_")
        .tempdir_in(target_dir)
        .unwrap()
}

fn write_file(path: &Path, source: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(path, source).unwrap();
}

fn analyze(root: &Path) -> AnalysisResult {
    CytoScnPy::default()
        .with_confidence(60)
        .with_tests(false)
        .analyze(root)
}

#[test]
fn star_import_without_all_resolves_public_source_symbols() {
    let dir = project_tempdir();
    let pkg = dir.path().join("pkg");
    write_file(&pkg.join("__init__.py"), "");
    write_file(
        &pkg.join("lib.py"),
        "pub_const = 41\n\ndef used_via_star():\n    return pub_const + 1\n",
    );
    write_file(
        &pkg.join("main.py"),
        "from pkg.lib import *\n\nresult = used_via_star() + pub_const\n",
    );

    let result = analyze(dir.path());

    assert!(
        !result
            .unused_functions
            .iter()
            .any(|def| def.full_name == "pkg.lib.used_via_star"),
        "a public function used through a star import must remain live"
    );
    assert!(
        !result
            .unused_variables
            .iter()
            .any(|def| def.full_name == "pkg.lib.pub_const"),
        "a public variable used through a star import must remain live"
    );
}

#[test]
fn public_name_fallback_propagates_through_star_import_chains() {
    let dir = project_tempdir();
    let pkg = dir.path().join("pkg");
    write_file(&pkg.join("__init__.py"), "");
    write_file(
        &pkg.join("base.py"),
        "def chained_function():\n    return 42\n",
    );
    write_file(&pkg.join("middle.py"), "from pkg.base import *\n");
    write_file(
        &pkg.join("main.py"),
        "from pkg.middle import *\n\nresult = chained_function()\n",
    );

    let result = analyze(dir.path());

    assert!(
        !result
            .unused_functions
            .iter()
            .any(|def| def.full_name == "pkg.base.chained_function"),
        "public fallback bindings must propagate through consecutive star imports"
    );
}

#[test]
fn src_layout_uses_the_python_package_name() {
    let dir = project_tempdir();
    let pkg = dir.path().join("src").join("pkg");
    write_file(&pkg.join("__init__.py"), "");
    write_file(&pkg.join("lib.py"), "MY_CONSTANT = 42\n");
    write_file(
        &pkg.join("main.py"),
        "from pkg.lib import MY_CONSTANT\n\nresult = MY_CONSTANT\n",
    );

    let result = analyze(dir.path());

    assert!(
        !result
            .unused_variables
            .iter()
            .any(|def| def.simple_name == "MY_CONSTANT"),
        "src/pkg/lib.py must be named pkg.lib so its imported constant resolves"
    );
    assert!(
        !result
            .unused_imports
            .iter()
            .any(|def| def.simple_name == "MY_CONSTANT"),
        "the used import binding must not be reported unused"
    );
}

#[test]
fn package_name_is_stable_when_scanning_a_subdirectory() {
    let dir = project_tempdir();
    let pkg = dir.path().join("pkg");
    let subpkg = pkg.join("subpkg");
    write_file(&pkg.join("__init__.py"), "");
    write_file(&subpkg.join("__init__.py"), "");
    write_file(&subpkg.join("lib.py"), "PACKAGE_VALUE = 42\n");
    write_file(
        &subpkg.join("consumer.py"),
        "from pkg.subpkg.lib import PACKAGE_VALUE\n\nresult = PACKAGE_VALUE\n",
    );

    let result = analyze(&subpkg);

    assert!(
        !result
            .unused_variables
            .iter()
            .any(|def| def.simple_name == "PACKAGE_VALUE"),
        "qualified names must retain pkg.subpkg when the scan starts inside the package"
    );
}

#[test]
fn explicit_empty_all_disables_public_name_fallback() {
    let dir = project_tempdir();
    write_file(
        &dir.path().join("lib.py"),
        "__all__ = []\n\ndef excluded_function():\n    return 42\n",
    );
    write_file(
        &dir.path().join("main.py"),
        "from lib import *\n\nresult = excluded_function()\n",
    );

    let result = analyze(dir.path());

    assert!(
        result
            .unused_functions
            .iter()
            .any(|def| def.full_name == "lib.excluded_function"),
        "an explicit empty __all__ must not fall back to public module names"
    );
}

#[test]
fn unused_import_does_not_keep_source_function_alive() {
    let dir = project_tempdir();
    write_file(
        &dir.path().join("lib.py"),
        "def dead_function():\n    return 42\n",
    );
    write_file(
        &dir.path().join("main.py"),
        "from lib import dead_function\n",
    );

    let result = analyze(dir.path());

    assert!(
        result
            .unused_imports
            .iter()
            .any(|def| def.full_name == "main.dead_function"),
        "the unused local import binding must be reported"
    );
    assert!(
        result
            .unused_functions
            .iter()
            .any(|def| def.full_name == "lib.dead_function"),
        "an unused import must not count as a use of its source function"
    );
}
