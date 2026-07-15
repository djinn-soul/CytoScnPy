use super::analysis::{analyze_dependencies, DepsOptions, DepsResult};
use super::declared::DeclaredDependency;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

fn options(roots: &[PathBuf]) -> DepsOptions<'_> {
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

fn write(dir: &Path, relative: &str, content: &str) {
    let path = dir.join(relative);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent dir");
    }
    fs::write(path, content).expect("write file");
}

/// Builds a project from `(relative path, content)` pairs and analyzes it,
/// treating `root` as the single analysis root.
fn analyze(files: &[(&str, &str)]) -> (TempDir, DepsResult) {
    analyze_from(files, "")
}

/// Same as [`analyze`], but the analysis root is `subdir` inside the project.
fn analyze_from(files: &[(&str, &str)], subdir: &str) -> (TempDir, DepsResult) {
    let dir = tempfile::tempdir().expect("tempdir");
    for (relative, content) in files {
        write(dir.path(), relative, content);
    }
    let roots = vec![dir.path().join(subdir)];
    let result = analyze_dependencies(&options(&roots));
    (dir, result)
}

fn unused_names(unused: &[DeclaredDependency]) -> Vec<&str> {
    unused.iter().map(|dep| dep.package_name.as_str()).collect()
}

const PYPROJECT_HEADER: &str = "[project]\nname = \"proj\"\nversion = \"0.1.0\"\n";

fn pyproject(dependencies: &str) -> String {
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

#[test]
fn extra_and_orphan_installed_packages_are_reported_from_the_venv() {
    let dir = tempfile::tempdir().expect("tempdir");
    write(dir.path(), "pyproject.toml", &pyproject("\"requests\""));
    write(dir.path(), "app.py", "import requests\n");

    // requests: declared. urllib3: undeclared but required by requests. bloat: neither.
    let site_packages = dir.path().join(".venv/Lib/site-packages");
    for (name, requires) in [
        ("requests", "Requires-Dist: urllib3\n"),
        ("urllib3", ""),
        ("bloat", ""),
    ] {
        let info = site_packages.join(format!("{name}-1.0.dist-info"));
        fs::create_dir_all(&info).expect("create dist-info");
        fs::write(
            info.join("METADATA"),
            format!("Name: {name}\nVersion: 1.0\n{requires}"),
        )
        .expect("write METADATA");
    }

    let roots = vec![dir.path().to_path_buf()];
    let mut opts = options(&roots);
    opts.show_extra = true;
    opts.show_orphans = true;
    let result = analyze_dependencies(&opts);

    let extra: Vec<&str> = result
        .extra_installed
        .iter()
        .map(|pkg| pkg.normalized_name.as_str())
        .collect();
    assert_eq!(
        extra,
        vec!["bloat", "urllib3"],
        "declared packages are not extra"
    );

    let orphans: Vec<&str> = result
        .orphan_installed
        .iter()
        .map(|pkg| pkg.normalized_name.as_str())
        .collect();
    assert_eq!(
        orphans,
        vec!["bloat"],
        "urllib3 is required by requests, so it is not an orphan"
    );
}

/// `requests` (declared) pulls in `urllib3` and `certifi`; `httpx` (declared,
/// unused) pulls in `anyio`, which nothing else needs.
const UV_LOCK: &str = "\
version = 1

[[package]]
name = \"requests\"
version = \"2.31.0\"
dependencies = [
  { name = \"urllib3\" },
  { name = \"certifi\" },
]

[[package]]
name = \"httpx\"
version = \"0.27.0\"
dependencies = [
  { name = \"anyio\" },
  { name = \"certifi\" },
]

[[package]]
name = \"urllib3\"
version = \"2.0.0\"

[[package]]
name = \"certifi\"
version = \"2024.1.1\"

[[package]]
name = \"anyio\"
version = \"4.0.0\"
";

#[test]
fn an_import_satisfied_only_by_a_transitive_lockfile_package_is_reported_as_transitive() {
    let (_dir, result) = analyze(&[
        ("pyproject.toml", &pyproject("\"requests\"")),
        ("uv.lock", UV_LOCK),
        ("app.py", "import requests\nimport urllib3\n"),
    ]);

    let transitive: Vec<&str> = result
        .transitive
        .iter()
        .map(|dep| dep.import_name.as_str())
        .collect();
    assert_eq!(transitive, vec!["urllib3"]);
    assert_eq!(result.transitive[0].package_name, "urllib3");
    assert!(
        !result.transitive[0].locations.is_empty(),
        "evidence is kept"
    );
    assert!(
        result.missing.is_empty(),
        "a transitively available import is not `missing`: {:?}",
        result.missing
    );
}

#[test]
fn removable_branches_exclude_packages_still_needed_by_other_declared_roots() {
    let (_dir, result) = analyze(&[
        ("pyproject.toml", &pyproject("\"requests\", \"httpx\"")),
        ("uv.lock", UV_LOCK),
        ("app.py", "import requests\n"),
    ]);

    assert_eq!(unused_names(&result.unused), vec!["httpx"]);
    assert_eq!(result.removable_branches.len(), 1);
    let branch = &result.removable_branches[0];
    assert_eq!(branch.root, "httpx");
    assert_eq!(
        branch.unique_transitive,
        vec!["anyio".to_owned()],
        "certifi is still required by the declared `requests`, so it is not removable"
    );
}

#[test]
fn impact_package_reports_the_branch_for_one_package_only() {
    let dir = tempfile::tempdir().expect("tempdir");
    write(
        dir.path(),
        "pyproject.toml",
        &pyproject("\"requests\", \"httpx\""),
    );
    write(dir.path(), "uv.lock", UV_LOCK);
    write(dir.path(), "app.py", "import requests\nimport httpx\n");

    let roots = vec![dir.path().to_path_buf()];
    let mut opts = options(&roots);
    opts.impact_package = Some("requests".to_owned());
    let result = analyze_dependencies(&opts);

    assert!(result.unused.is_empty(), "both packages are imported");
    assert_eq!(
        result.removable_branches.len(),
        1,
        "impact is reported for the requested package even when it is used"
    );
    assert_eq!(result.removable_branches[0].root, "requests");
    assert_eq!(
        result.removable_branches[0].unique_transitive,
        vec!["urllib3".to_owned()],
        "certifi is shared with httpx"
    );
}

#[test]
fn a_setuptools_project_declares_its_dependencies_in_setup_py() {
    let (_dir, result) = analyze(&[
        (
            "setup.py",
            "from setuptools import setup\nsetup(name='proj', install_requires=['requests'])\n",
        ),
        ("app.py", "import requests\n"),
    ]);

    assert!(
        result.missing.is_empty(),
        "install_requires is a declaration, and setup.py's own build imports are not \
         dependencies: {:?}",
        result.missing
    );
    assert!(result.unused.is_empty(), "requests is imported");
}

#[test]
fn a_setuptools_project_declares_its_dependencies_in_setup_cfg() {
    let (_dir, result) = analyze(&[
        (
            "setup.cfg",
            "[options]\ninstall_requires =\n    requests\n    flask\n",
        ),
        ("app.py", "import requests\n"),
    ]);

    assert!(result.missing.is_empty(), "missing: {:?}", result.missing);
    assert_eq!(unused_names(&result.unused), vec!["flask"]);
}

/// Installs `name` into a fake venv under `dir`, recording the import names it
/// provides in `top_level.txt` the way a real installer does.
fn install(dir: &Path, name: &str, top_level: &[&str], requires: &[&str]) {
    let info = dir
        .join(".venv/Lib/site-packages")
        .join(format!("{name}-1.0.dist-info"));
    fs::create_dir_all(&info).expect("create dist-info");

    let mut metadata = format!("Name: {name}\nVersion: 1.0\n");
    for req in requires {
        metadata.push_str(&format!("Requires-Dist: {req}\n"));
    }
    fs::write(info.join("METADATA"), metadata).expect("write METADATA");

    if !top_level.is_empty() {
        fs::write(
            info.join("top_level.txt"),
            format!("{}\n", top_level.join("\n")),
        )
        .expect("write top_level.txt");
    }
}

#[test]
fn import_names_come_from_the_installed_environment_not_a_guess_table() {
    // `python-slugify` imports as `slugify`. It is in no built-in table, so before
    // the environment was consulted this was both "unused" and "missing".
    let dir = tempfile::tempdir().expect("tempdir");
    write(
        dir.path(),
        "pyproject.toml",
        &pyproject("\"python-slugify\""),
    );
    write(dir.path(), "app.py", "from slugify import slugify\n");
    install(dir.path(), "python_slugify", &["slugify"], &[]);

    let roots = vec![dir.path().to_path_buf()];
    let result = analyze_dependencies(&options(&roots));

    assert!(
        result.unused.is_empty(),
        "top_level.txt says python-slugify provides `slugify`: {:?}",
        unused_names(&result.unused)
    );
    assert!(result.missing.is_empty(), "missing: {:?}", result.missing);
}

#[test]
fn without_an_environment_the_builtin_table_still_applies() {
    let (_dir, result) = analyze(&[
        (
            "pyproject.toml",
            &pyproject("\"pyyaml\", \"python-slugify\""),
        ),
        ("app.py", "import yaml\nfrom slugify import slugify\n"),
    ]);

    assert_eq!(
        unused_names(&result.unused),
        vec!["python-slugify"],
        "pyyaml→yaml is in the built-in table; python-slugify can only be resolved \
         from an environment"
    );
}

#[test]
fn the_environment_mapping_resolves_imports_back_to_their_distribution() {
    // `slugify` is imported but undeclared. The environment knows which
    // distribution provides it, so the orphan report must not treat that
    // distribution as unused-and-unreferenced.
    let dir = tempfile::tempdir().expect("tempdir");
    write(dir.path(), "pyproject.toml", &pyproject(""));
    write(dir.path(), "app.py", "from slugify import slugify\n");
    install(dir.path(), "python_slugify", &["slugify"], &[]);
    install(dir.path(), "bloat", &["bloat"], &[]);

    let roots = vec![dir.path().to_path_buf()];
    let mut opts = options(&roots);
    opts.show_orphans = true;
    let result = analyze_dependencies(&opts);

    let orphans: Vec<&str> = result
        .orphan_installed
        .iter()
        .map(|pkg| pkg.normalized_name.as_str())
        .collect();
    assert_eq!(
        orphans,
        vec!["bloat"],
        "python-slugify is imported as `slugify`, so it is not an orphan"
    );
}

#[test]
fn a_user_package_mapping_overrides_the_environment() {
    let dir = tempfile::tempdir().expect("tempdir");
    write(dir.path(), "pyproject.toml", &pyproject("\"weird-dist\""));
    write(dir.path(), "app.py", "import chosen\n");
    install(dir.path(), "weird_dist", &["ignored"], &[]);

    let mut mapping = rustc_hash::FxHashMap::default();
    mapping.insert("weird-dist".to_owned(), vec!["chosen".to_owned()]);

    let roots = vec![dir.path().to_path_buf()];
    let mut opts = options(&roots);
    opts.package_mapping = Some(&mapping);
    let result = analyze_dependencies(&opts);

    assert!(
        result.unused.is_empty(),
        "the configured mapping wins over top_level.txt: {:?}",
        unused_names(&result.unused)
    );
}

#[test]
fn a_user_mapped_import_resolves_back_to_the_declared_distribution() {
    let dir = tempfile::tempdir().expect("tempdir");
    write(
        dir.path(),
        "pyproject.toml",
        &pyproject("\"my-internal-lib\""),
    );
    write(dir.path(), "app.py", "import mylib\n");

    let mut mapping = rustc_hash::FxHashMap::default();
    mapping.insert("my-internal-lib".to_owned(), vec!["mylib".to_owned()]);

    let roots = vec![dir.path().to_path_buf()];
    let mut opts = options(&roots);
    opts.package_mapping = Some(&mapping);
    let result = analyze_dependencies(&opts);

    assert!(
        result.missing.is_empty(),
        "the mapping says `mylib` comes from the declared `my-internal-lib`: {:?}",
        result.missing
    );
    assert!(result.unused.is_empty(), "my-internal-lib is imported");
}

#[test]
fn a_namespace_import_is_not_transitive_when_a_declared_dist_publishes_into_it() {
    let dir = tempfile::tempdir().expect("tempdir");
    write(
        dir.path(),
        "pyproject.toml",
        &pyproject("\"google-cloud-storage\""),
    );
    write(dir.path(), "app.py", "from google.cloud import storage\n");
    write(
        dir.path(),
        "uv.lock",
        "version = 1\n\n\
         [[package]]\n\
         name = \"google-cloud-storage\"\n\
         version = \"2.0.0\"\n\
         dependencies = [\n  { name = \"google-api-core\" },\n]\n\n\
         [[package]]\n\
         name = \"google-api-core\"\n\
         version = \"2.0.0\"\n",
    );
    // Both distributions record `google` as their import root, and the reverse
    // map can pick either one for the shared namespace.
    install(dir.path(), "google_api_core", &["google"], &[]);
    install(dir.path(), "google_cloud_storage", &["google"], &[]);

    let roots = vec![dir.path().to_path_buf()];
    let result = analyze_dependencies(&options(&roots));

    assert!(
        result.transitive.is_empty(),
        "`google` is published by the declared google-cloud-storage: {:?}",
        result
            .transitive
            .iter()
            .map(|dep| dep.package_name.as_str())
            .collect::<Vec<_>>()
    );
    assert!(result.missing.is_empty(), "google is declared");
}

#[test]
fn an_imported_undeclared_package_is_missing_with_source_evidence() {
    let (_dir, result) = analyze(&[
        ("pyproject.toml", &pyproject("")),
        ("app.py", "import os\nimport requests\n"),
    ]);

    assert_eq!(result.missing, vec!["requests".to_owned()]);
    let detail = &result.missing_details[0];
    assert_eq!(detail.locations.len(), 1);
    assert_eq!(detail.locations[0].line, 2);
}
