//! Regression tests for exported requirements files.
#![allow(clippy::unwrap_used)]

use cytoscnpy::entry_point::run_with_args_to;
use std::fs;
use tempfile::tempdir;

fn run_deps_command(args: Vec<String>) -> (i32, String) {
    let mut buffer = Vec::new();
    let code = run_with_args_to(args, &mut buffer).unwrap_or(1);
    let output = String::from_utf8_lossy(&buffer).into_owned();
    (code, output)
}

#[test]
fn exported_requirements_pins_do_not_fail_unused_gate() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let root = dir.path();

    fs::write(
        root.join("pyproject.toml"),
        r#"[project]
name = "test-pkg"
version = "0.1.0"
dependencies = [
    "requests>=2.0.0",
]
"#,
    )?;
    fs::write(
        root.join("requirements.txt"),
        r"certifi==2024.2.2
charset-normalizer==3.3.2
idna==3.6
requests==2.31.0
urllib3==2.2.1
",
    )?;
    fs::write(root.join("main.py"), "import requests\n")?;

    let (code, output) = run_deps_command(vec![
        "deps".to_owned(),
        root.to_string_lossy().into_owned(),
        "--fail-on-unused".to_owned(),
    ]);

    assert_eq!(code, 0, "{output}");
    assert!(
        !output.contains("Unused Dependencies"),
        "exported pins should not be treated as unused direct deps: {output}"
    );
    Ok(())
}
