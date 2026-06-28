//! Regression tests for dependency missing-gate namespace false positives.
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
fn future_and_azure_namespace_imports_do_not_fail_missing_gate() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let root = dir.path();

    fs::write(
        root.join("pyproject.toml"),
        r#"[project]
name = "test-pkg"
version = "0.1.0"
dependencies = [
    "azure-functions>1.0.0",
    "azure-identity>=1.25.1",
]
"#,
    )?;
    fs::write(
        root.join("main.py"),
        r"from __future__ import annotations
import azure.functions as func
from azure.identity import DefaultAzureCredential
",
    )?;

    let (code, output) = run_deps_command(vec![
        "deps".to_owned(),
        root.to_string_lossy().into_owned(),
        "--fail-on-missing".to_owned(),
    ]);

    assert_eq!(code, 0, "{output}");
    assert!(
        !output.contains("Missing Dependencies"),
        "__future__ or azure should not be reported missing: {output}"
    );
    Ok(())
}
