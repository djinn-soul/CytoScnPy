//! Dependency analysis tests for virtualenv, lockfile, and setuptools inputs.

use super::*;

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
