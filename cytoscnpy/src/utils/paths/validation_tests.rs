use super::*;
use std::fs;
use std::path::Path;
use tempfile::tempdir;

#[test]
fn test_validate_path_within_root() -> anyhow::Result<()> {
    let temp = tempdir()?;
    let root = temp.path();

    // Create nested structure
    let inside = root.join("subdir/file.py");
    fs::create_dir_all(
        inside
            .parent()
            .ok_or_else(|| anyhow::anyhow!("No parent"))?,
    )?;
    fs::write(&inside, "# test")?;

    // Test valid path (inside root)
    assert!(validate_path_within_root(&inside, root).is_ok());

    // Test invalid path (outside root via ..)
    let outside = root.join("../outside.py");
    assert!(validate_path_within_root(&outside, root).is_err());

    // Test path traversal
    let traversal = root.join("subdir/../../etc/passwd");
    assert!(validate_path_within_root(&traversal, root).is_err());

    Ok(())
}

/// Helper to run test in a specific directory
fn run_in_dir<F>(dir: &Path, f: F) -> anyhow::Result<()>
where
    F: FnOnce() -> anyhow::Result<()>,
{
    let original = std::env::current_dir()?;
    std::env::set_current_dir(dir)?;
    let result = f();
    std::env::set_current_dir(original)?;
    result
}

#[test]
fn test_validate_output_path_security() -> anyhow::Result<()> {
    let temp = tempdir()?;
    let root = temp.path();
    fs::create_dir_all(root.join("subdir"))?;

    run_in_dir(root, || {
        // Test valid relative paths
        assert!(validate_output_path(Path::new("./report.json"), None).is_ok());
        assert!(validate_output_path(Path::new("subdir/output.json"), None).is_ok());

        // Test path traversal attempts
        assert!(validate_output_path(Path::new("../outside.json"), None).is_err());
        assert!(validate_output_path(Path::new("subdir/../../escape.json"), None).is_err());

        // Test with explicit root
        assert!(validate_output_path(Path::new("./report.json"), Some(root)).is_ok());
        assert!(validate_output_path(Path::new("../outside.json"), Some(root)).is_err());

        Ok(())
    })
}
