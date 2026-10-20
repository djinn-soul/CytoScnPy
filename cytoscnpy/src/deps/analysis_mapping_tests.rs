//! Dependency analysis tests for import-name mapping from installed distributions.

use super::*;

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
