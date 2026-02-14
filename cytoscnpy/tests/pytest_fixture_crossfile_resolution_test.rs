//! Integration tests for cross-file pytest fixture resolution.
#![allow(clippy::unwrap_used)]

use cytoscnpy::analyzer::CytoScnPy;
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use tempfile::TempDir;

fn project_tempdir() -> TempDir {
    let mut target_dir = std::env::current_dir().unwrap();
    target_dir.push("target");
    target_dir.push("test-pytest-fixture-crossfile");
    fs::create_dir_all(&target_dir).unwrap();
    tempfile::Builder::new()
        .prefix("pytest_fixture_crossfile_")
        .tempdir_in(target_dir)
        .unwrap()
}

fn write_file(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    let mut file = File::create(path).unwrap();
    write!(file, "{content}").unwrap();
}

fn analyze(dir: &Path) -> cytoscnpy::analyzer::AnalysisResult {
    let mut analyzer = CytoScnPy::default().with_confidence(1).with_tests(false);
    analyzer.analyze(dir)
}

#[test]
fn conftest_fixture_used_by_test_parameter_is_not_reported_unused() {
    let dir = project_tempdir();
    write_file(
        &dir.path().join("conftest.py"),
        r#"
import pytest

@pytest.fixture
def client():
    return object()
"#,
    );
    write_file(
        &dir.path().join("tests/test_api.py"),
        r#"
def test_api(client):
    assert client is not None
"#,
    );

    let result = analyze(dir.path());
    assert!(
        !result
            .unused_functions
            .iter()
            .any(|f| f.simple_name == "client"),
        "conftest fixture `client` should be marked as used across files"
    );
}

#[test]
fn conftest_fixture_used_by_usefixtures_is_not_reported_unused() {
    let dir = project_tempdir();
    write_file(
        &dir.path().join("conftest.py"),
        r#"
import pytest

@pytest.fixture
def db():
    return {}
"#,
    );
    write_file(
        &dir.path().join("tests/test_db.py"),
        r#"
import pytest

@pytest.mark.usefixtures("db")
def test_db():
    assert True
"#,
    );

    let result = analyze(dir.path());
    assert!(
        !result
            .unused_functions
            .iter()
            .any(|f| f.simple_name == "db"),
        "fixture referenced by @usefixtures across files should be marked used"
    );
}

#[test]
fn nearest_conftest_fixture_wins_for_shadowed_names() {
    let dir = project_tempdir();
    write_file(
        &dir.path().join("conftest.py"),
        r#"
import pytest

@pytest.fixture
def data():
    return "root"
"#,
    );
    write_file(
        &dir.path().join("tests/conftest.py"),
        r#"
import pytest

@pytest.fixture
def data():
    return "nested"
"#,
    );
    write_file(
        &dir.path().join("tests/test_data.py"),
        r#"
def test_data(data):
    assert data == "nested"
"#,
    );

    let result = analyze(dir.path());
    let data_unused: Vec<_> = result
        .unused_functions
        .iter()
        .filter(|f| f.simple_name == "data")
        .collect();
    assert_eq!(
        data_unused.len(),
        1,
        "only one shadowed `data` fixture should remain unused"
    );
    assert_eq!(
        data_unused[0].file.parent().unwrap(),
        dir.path(),
        "root conftest fixture should stay unused when nested conftest shadows it"
    );
}

#[test]
fn usefixtures_does_not_mark_non_fixture_production_symbol_as_used() {
    let dir = project_tempdir();
    write_file(
        &dir.path().join("main.py"),
        r#"
def client():
    return "unused production helper"
"#,
    );
    write_file(
        &dir.path().join("tests/test_usefixtures.py"),
        r#"
import pytest

@pytest.mark.usefixtures("client")
def test_anything():
    assert True
"#,
    );

    let result = analyze(dir.path());
    assert!(
        result
            .unused_functions
            .iter()
            .any(|f| f.simple_name == "client" && f.file.ends_with("main.py")),
        "non-fixture production function must not be marked used by @usefixtures alone"
    );
}

#[test]
fn pytest_plugins_fixture_is_resolved_across_modules() {
    let dir = project_tempdir();
    write_file(
        &dir.path().join("conftest.py"),
        r#"
pytest_plugins = ["tests.plugins.fixtures"]
"#,
    );
    write_file(
        &dir.path().join("tests/plugins/fixtures.py"),
        r#"
import pytest

@pytest.fixture
def token():
    return "abc"
"#,
    );
    write_file(
        &dir.path().join("tests/test_token.py"),
        r#"
def test_token(token):
    assert token == "abc"
"#,
    );

    let result = analyze(dir.path());
    assert!(
        !result.unused_functions.iter().any(|f| {
            f.simple_name == "token" && f.file.ends_with("tests\\plugins\\fixtures.py")
        }),
        "fixture declared via pytest_plugins should be resolved and marked used"
    );
}

#[test]
fn fixture_alias_name_in_decorator_is_resolved() {
    let dir = project_tempdir();
    write_file(
        &dir.path().join("conftest.py"),
        r#"
import pytest

@pytest.fixture(name="api_client")
def client_impl():
    return object()
"#,
    );
    write_file(
        &dir.path().join("tests/test_alias.py"),
        r#"
def test_alias(api_client):
    assert api_client is not None
"#,
    );

    let result = analyze(dir.path());
    assert!(
        !result
            .unused_functions
            .iter()
            .any(|f| f.simple_name == "client_impl"),
        "fixture alias name from decorator should mark implementation as used"
    );
}

#[test]
fn relative_imported_fixture_module_is_resolved() {
    let dir = project_tempdir();
    write_file(&dir.path().join("tests/__init__.py"), "");
    write_file(
        &dir.path().join("tests/fixtures.py"),
        r#"
import pytest

@pytest.fixture
def db():
    return {}
"#,
    );
    write_file(
        &dir.path().join("tests/test_db.py"),
        r#"
from .fixtures import db

def test_db(db):
    assert db == {}
"#,
    );

    let result = analyze(dir.path());
    assert!(
        !result
            .unused_functions
            .iter()
            .any(|f| { f.simple_name == "db" && f.file.ends_with("tests\\fixtures.py") }),
        "fixture imported via relative module path should be resolved and marked used"
    );
}
