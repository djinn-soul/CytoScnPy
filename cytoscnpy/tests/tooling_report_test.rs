//! End-to-end checks for configuration inventory in terminal and JSON reports.
#![allow(clippy::unwrap_used)]

use std::fs;
use std::io::Cursor;

use cytoscnpy::entry_point;
use tempfile::TempDir;

fn deslop(root: &std::path::Path, flags: &[&str]) -> String {
    let mut args = vec!["deslop".to_owned(), root.display().to_string()];
    args.extend(flags.iter().map(|flag| (*flag).to_owned()));
    let mut output = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(args, &mut output).unwrap();
    assert_eq!(code, 0);
    String::from_utf8(output.into_inner()).unwrap()
}

#[test]
fn reports_all_tooling_with_paths_and_reliable_json_signals() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();
    fs::write(root.join("main.py"), "def answer():\n    return 42\n").unwrap();
    fs::write(
        root.join("pyproject.toml"),
        "[project]\nname = 'sample'\nversion = '0.1.0'\n\
         [tool.ruff]\nline-length = 88\n\
         [tool.mypy]\nstrict = true\n\
         [tool.ty]\n\
         [tool.pytest.ini_options]\ntestpaths = ['tests']\n",
    )
    .unwrap();
    fs::write(root.join("ruff.toml"), "line-length = 88\n").unwrap();
    fs::write(root.join("mypy.ini"), "[mypy]\nstrict = true\n").unwrap();
    fs::write(root.join("pytest.ini"), "[pytest]\n").unwrap();
    fs::write(root.join("pdm.lock"), "lock_version = '4.0'\n").unwrap();
    fs::write(root.join("requirements-dev.txt"), "pytest\n").unwrap();
    fs::write(root.join("bitbucket-pipelines.yml"), "pipelines: {}\n").unwrap();
    fs::create_dir(root.join("docs")).unwrap();
    fs::write(root.join("docs/ARCHITECTURE.md"), "# Design\n").unwrap();

    let json: serde_json::Value =
        serde_json::from_str(&deslop(root, &["--json", "--no-git"])).unwrap();
    let configs = json["health"][0]["configs"].as_array().unwrap();
    let has_config = |category: &str, name: &str, suffix: &str| {
        configs.iter().any(|config| {
            config["category"] == category
                && config["name"] == name
                && config["path"].as_str().unwrap().ends_with(suffix)
        })
    };
    assert!(has_config("Formatter", "ruff.toml", "ruff.toml"));
    assert!(has_config(
        "Formatter",
        "Ruff (pyproject.toml)",
        "pyproject.toml"
    ));
    assert!(has_config("TypeChecker", "mypy.ini", "mypy.ini"));
    assert!(has_config(
        "TypeChecker",
        "mypy (pyproject.toml)",
        "pyproject.toml"
    ));
    assert!(has_config(
        "TypeChecker",
        "ty (pyproject.toml)",
        "pyproject.toml"
    ));
    assert!(has_config(
        "TestFramework",
        "pytest (pyproject.toml)",
        "pyproject.toml"
    ));
    assert!(has_config("Lockfile", "pdm.lock", "pdm.lock"));
    assert!(has_config(
        "CI",
        "bitbucket-pipelines.yml",
        "bitbucket-pipelines.yml"
    ));
    assert_eq!(json["health"][0]["reliability"]["has_type_checker"], true);
    assert_eq!(json["health"][0]["reliability"]["has_ci"], true);
    assert_eq!(json["health"][0]["reliability"]["has_lockfile"], true);

    let verbose = deslop(root, &["--verbose", "--no-git"]);
    assert!(verbose.contains("pyproject.toml (Configured under [tool.mypy])"));
    assert!(verbose.contains("docs/ARCHITECTURE.md"));
    assert!(verbose.contains("requirements-dev.txt"));

    let brief = deslop(root, &["--no-git"]);
    assert!(brief.contains("Tooling Detected:"));
    assert!(brief.contains("more; see --verbose or --json"));
    assert!(brief.contains("[GATE PASSED]"));
    assert!(!brief.contains("=== Detected Tooling & Configuration Inventory ==="));
}

#[test]
fn directories_named_like_config_files_do_not_claim_tooling() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();
    fs::write(root.join("main.py"), "pass\n").unwrap();
    fs::create_dir(root.join("mypy.ini")).unwrap();
    fs::create_dir(root.join("pdm.lock")).unwrap();

    let json: serde_json::Value =
        serde_json::from_str(&deslop(root, &["--json", "--no-git"])).unwrap();
    let health = &json["health"][0];
    assert_eq!(health["reliability"]["has_type_checker"], false);
    assert_eq!(health["reliability"]["has_lockfile"], false);
    assert!(health["configs"].as_array().unwrap().is_empty());
}
