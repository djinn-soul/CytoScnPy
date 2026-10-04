use super::*;
use std::fs;
use tempfile::tempdir;

#[test]
fn test_collect_python_files_exclusion() -> anyhow::Result<()> {
    let temp = tempdir()?;
    let root = temp.path();

    // Create Python files
    fs::write(root.join("main.py"), "# main")?;
    fs::write(root.join("app.py"), "# app")?;

    // Create excluded directories with Python files
    fs::create_dir_all(root.join(".venv"))?;
    fs::write(root.join(".venv/lib.py"), "# venv lib")?;

    fs::create_dir_all(root.join("node_modules"))?;
    fs::write(root.join("node_modules/script.py"), "# node")?;

    fs::create_dir_all(root.join("__pycache__"))?;
    fs::write(root.join("__pycache__/cached.py"), "# cached")?;

    // Create valid subdirectory
    fs::create_dir_all(root.join("src"))?;
    fs::write(root.join("src/module.py"), "# module")?;

    let (files, _) = collect_python_files_gitignore(root, &[], &[], false, false);

    // Should find main.py, app.py, src/module.py
    // Should NOT find .venv/lib.py, node_modules/script.py, __pycache__/cached.py
    assert_eq!(files.len(), 3);

    let file_names: Vec<_> = files
        .iter()
        .filter_map(|p| p.file_name())
        .filter_map(|f| f.to_str())
        .collect();

    assert!(file_names.contains(&"main.py"));
    assert!(file_names.contains(&"app.py"));
    assert!(file_names.contains(&"module.py"));
    assert!(!file_names.contains(&"lib.py"));
    assert!(!file_names.contains(&"script.py"));
    assert!(!file_names.contains(&"cached.py"));

    Ok(())
}

#[test]
fn test_collect_python_files_force_include() -> anyhow::Result<()> {
    let temp = tempdir()?;
    let root = temp.path();

    // Create a normally-excluded directory that we want to force-include
    fs::create_dir_all(root.join("tests"))?;
    fs::write(root.join("tests/test_main.py"), "# test")?;

    // Force-include "tests" (which might be in DEFAULT_EXCLUDE_FOLDERS)
    // Note: tests is not actually in defaults, but this tests the mechanism
    let (files, _) = collect_python_files_gitignore(root, &[], &["tests".to_owned()], false, false);

    assert_eq!(files.len(), 1);

    Ok(())
}

#[test]
fn test_collect_python_files_no_substring_unexclude() -> anyhow::Result<()> {
    let temp = tempdir()?;
    let root = temp.path();

    // Create structure where "tests" is force-included but ".venv" has "tests" substring
    fs::create_dir_all(root.join("tests"))?;
    fs::write(root.join("tests/test_file.py"), "# test")?;

    fs::create_dir_all(root.join(".venv/site-packages/tests"))?;
    fs::write(
        root.join(".venv/site-packages/tests/internal.py"),
        "# internal",
    )?;

    // Force include "tests" but .venv should still be excluded
    let (files, _) = collect_python_files_gitignore(root, &[], &["tests".to_owned()], false, false);

    // Should find tests/test_file.py but NOT .venv/site-packages/tests/internal.py
    assert_eq!(files.len(), 1);

    let file_names: Vec<_> = files
        .iter()
        .filter_map(|p| p.file_name())
        .filter_map(|f| f.to_str())
        .collect();

    assert!(file_names.contains(&"test_file.py"));
    assert!(!file_names.contains(&"internal.py"));

    Ok(())
}
