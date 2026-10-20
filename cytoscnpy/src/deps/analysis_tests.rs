use super::analysis::{analyze_dependencies, DepsOptions, DepsResult};
use super::declared::DeclaredDependency;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

#[path = "analysis_env_tests.rs"]
mod env;
#[path = "analysis_mapping_tests.rs"]
mod mapping;

pub(super) fn options(roots: &[PathBuf]) -> DepsOptions<'_> {
    DepsOptions {
        roots,
        exclude: &[],
        requirements: None,
        ignore_unused: &[],
        ignore_missing: &[],
        verbose: false,
        json: false,
        package_mapping: None,
        venv_path: None,
        lockfile_path: None,
        show_extra: false,
        show_orphans: false,
        impact_package: None,
        include_dev_unused: false,
    }
}

pub(super) fn write(dir: &Path, relative: &str, content: &str) {
    let path = dir.join(relative);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent dir");
    }
    fs::write(path, content).expect("write file");
}

/// Builds a project from `(relative path, content)` pairs and analyzes it,
/// treating `root` as the single analysis root.
pub(super) fn analyze(files: &[(&str, &str)]) -> (TempDir, DepsResult) {
    analyze_from(files, "")
}

/// Same as [`analyze`], but the analysis root is `subdir` inside the project.
pub(super) fn analyze_from(files: &[(&str, &str)], subdir: &str) -> (TempDir, DepsResult) {
    let dir = tempfile::tempdir().expect("tempdir");
    for (relative, content) in files {
        write(dir.path(), relative, content);
    }
    let roots = vec![dir.path().join(subdir)];
    let result = analyze_dependencies(&options(&roots));
    (dir, result)
}

pub(super) fn unused_names(unused: &[DeclaredDependency]) -> Vec<&str> {
    unused.iter().map(|dep| dep.package_name.as_str()).collect()
}

const PYPROJECT_HEADER: &str = "[project]\nname = \"proj\"\nversion = \"0.1.0\"\n";

pub(super) fn pyproject(dependencies: &str) -> String {
    format!("{PYPROJECT_HEADER}dependencies = [{dependencies}]\n")
}

#[test]
fn namespace_distribution_used_through_its_namespace_root_is_not_unused() {
    let (_dir, result) = analyze(&[
        (
            "pyproject.toml",
            &pyproject("\"google-cloud-storage\", \"azure-storage-blob\""),
        ),
        (
            "app.py",
            "from google.cloud import storage\nfrom azure.storage.blob import BlobClient\n",
        ),
    ]);

    assert!(
        result.unused.is_empty(),
        "namespace dists imported via their root must not be unused: {:?}",
        unused_names(&result.unused)
    );
    assert!(result.missing.is_empty(), "missing: {:?}", result.missing);
}

#[test]
fn dotted_distribution_name_matches_its_namespace_import() {
    let (_dir, result) = analyze(&[
        ("pyproject.toml", &pyproject("\"ruamel.yaml\"")),
        ("app.py", "from ruamel.yaml import YAML\n"),
    ]);

    assert!(
        result.unused.is_empty(),
        "ruamel.yaml is used: {:?}",
        unused_names(&result.unused)
    );
    assert!(
        result.missing.is_empty(),
        "`ruamel` is provided by the declared ruamel.yaml: {:?}",
        result.missing
    );
}

#[test]
fn unrelated_distribution_sharing_a_name_prefix_is_still_unused() {
    // `googlemaps` is not a `google` namespace dist, so importing `google`
    // must not mark it used.
    let (_dir, result) = analyze(&[
        ("pyproject.toml", &pyproject("\"googlemaps\"")),
        ("app.py", "from google.cloud import storage\n"),
    ]);

    assert_eq!(unused_names(&result.unused), vec!["googlemaps"]);
}

#[test]
fn first_party_package_under_src_layout_is_not_missing() {
    let (_dir, result) = analyze(&[
        ("pyproject.toml", &pyproject("")),
        ("src/mypkg/__init__.py", ""),
        ("tests/test_app.py", "import mypkg\n"),
    ]);

    assert!(
        result.missing.is_empty(),
        "src-layout package is first-party: {:?}",
        result.missing
    );
}

#[test]
fn declarations_are_found_when_analyzing_a_subdirectory() {
    let (_dir, result) = analyze_from(
        &[
            ("pyproject.toml", &pyproject("\"requests\"")),
            ("src/app.py", "import requests\n"),
        ],
        "src",
    );

    assert!(
        result.missing.is_empty(),
        "pyproject at the project root must still be read: {:?}",
        result.missing
    );
    assert!(result.unused.is_empty(), "requests is imported");
}

#[test]
fn dependency_imported_only_under_type_checking_is_not_unused_nor_missing() {
    let source = "from typing import TYPE_CHECKING\n\
                  if TYPE_CHECKING:\n    import pandas\n\n\
                  def f(df: 'pandas.DataFrame') -> None: ...\n";
    let (_dir, result) = analyze(&[
        ("pyproject.toml", &pyproject("\"pandas\"")),
        ("app.py", source),
    ]);

    assert!(
        result.unused.is_empty(),
        "a type-only import still uses the dependency: {:?}",
        unused_names(&result.unused)
    );
    assert!(
        result.missing.is_empty(),
        "type-only imports are not runtime imports: {:?}",
        result.missing
    );
}

#[test]
fn undeclared_type_checking_import_is_not_reported_as_missing() {
    let source = "from typing import TYPE_CHECKING\nif TYPE_CHECKING:\n    import pandas\n";
    let (_dir, result) = analyze(&[("pyproject.toml", &pyproject("")), ("app.py", source)]);

    assert!(
        result.missing.is_empty(),
        "type-only imports never make a dependency missing: {:?}",
        result.missing
    );
}

#[test]
fn dev_dependency_with_several_import_names_is_reported_once() {
    let manifest =
        format!("{PYPROJECT_HEADER}dependencies = []\n\n[dependency-groups]\ndev = [\"attrs\"]\n");
    let (_dir, result) = analyze(&[
        ("pyproject.toml", &manifest),
        ("app.py", "import attr\nimport attrs\n"),
    ]);

    assert_eq!(
        result.dev_in_production.len(),
        1,
        "attrs maps to two import names but is one dependency: {:?}",
        result
            .dev_in_production
            .iter()
            .map(|f| f.import_name.clone())
            .collect::<Vec<_>>()
    );
    let finding = &result.dev_in_production[0];
    assert_eq!(finding.dependency.package_name, "attrs");
    assert_eq!(
        finding.locations.len(),
        2,
        "evidence from both import names is kept"
    );
}

#[test]
fn requirements_include_directives_are_followed() {
    let (_dir, result) = analyze(&[
        ("requirements.txt", "requests\n"),
        ("requirements-dev.txt", "-r requirements.txt\npytest\n"),
        ("app.py", "import requests\nimport pytest\n"),
    ]);

    assert!(result.missing.is_empty(), "missing: {:?}", result.missing);
    assert!(
        result
            .unused
            .iter()
            .all(|dep| dep.package_name != "requests"),
        "requests is imported"
    );
}

#[test]
fn requirements_include_cycles_terminate() {
    let (_dir, result) = analyze(&[
        ("requirements.txt", "-r requirements-dev.txt\nrequests\n"),
        ("requirements-dev.txt", "-r requirements.txt\npytest\n"),
        ("app.py", "import requests\n"),
    ]);

    assert!(result.missing.is_empty(), "missing: {:?}", result.missing);
}

#[test]
fn a_declared_dependency_that_is_never_imported_is_still_unused() {
    let (_dir, result) = analyze(&[
        ("pyproject.toml", &pyproject("\"requests\", \"httpx\"")),
        ("app.py", "import requests\n"),
    ]);

    assert_eq!(unused_names(&result.unused), vec!["httpx"]);
}

#[test]
fn namespace_and_extension_modules_count_as_first_party() {
    // A PEP 420 namespace package (no __init__.py), a bare directory whose only
    // Python content is a nested package, a single-file module, a stub-only
    // module, and a compiled extension module are all first-party.
    let (_dir, result) = analyze(&[
        ("pyproject.toml", &pyproject("")),
        ("nspkg/mod.py", ""),
        ("outer/inner/__init__.py", ""),
        ("single.py", ""),
        ("stubbed.pyi", ""),
        ("native.pyd", ""),
        (
            "app.py",
            "import nspkg\nimport outer\nimport single\nimport stubbed\nimport native\n",
        ),
    ]);

    assert!(
        result.missing.is_empty(),
        "namespace, nested, single-file, stub and extension modules are first-party: {:?}",
        result.missing
    );
}

#[test]
fn an_empty_directory_is_not_a_local_package() {
    let dir = tempfile::tempdir().expect("tempdir");
    write(dir.path(), "pyproject.toml", &pyproject(""));
    write(dir.path(), "app.py", "import ghost\n");
    fs::create_dir_all(dir.path().join("ghost")).expect("create empty dir");

    let roots = vec![dir.path().to_path_buf()];
    let result = analyze_dependencies(&options(&roots));

    assert_eq!(
        result.missing,
        vec!["ghost".to_owned()],
        "a directory with no Python content is not a package"
    );
}
