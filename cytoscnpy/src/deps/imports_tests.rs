use super::*;
use std::fs;
use tempfile::tempdir;

#[test]
fn test_extract_imports_simple() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let file_path = dir.path().join("test.py");
    fs::write(
        &file_path,
        "import os\nfrom sys import path\nimport requests.sessions\n",
    )?;

    let imports = extract_imports(&[dir.path().to_path_buf()], &[], false);
    assert!(imports.contains("os"));
    assert!(imports.contains("sys"));
    assert!(imports.contains("requests"));
    assert_eq!(imports.len(), 3);
    Ok(())
}

#[test]
fn test_extract_imports_skips_relative() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let file_path = dir.path().join("test.py");
    fs::write(
        &file_path,
        "from . import local\nfrom ..parent import other\n",
    )?;

    let imports = extract_imports(&[dir.path().to_path_buf()], &[], false);
    assert!(imports.is_empty());
    Ok(())
}

#[test]
fn test_extract_imports_nested_in_function() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let file_path = dir.path().join("test.py");
    fs::write(
        &file_path,
        "def foo():\n    import json\n    from pathlib import Path\n",
    )?;

    let imports = extract_imports(&[dir.path().to_path_buf()], &[], false);
    assert!(imports.contains("json"));
    assert!(imports.contains("pathlib"));
    Ok(())
}

#[test]
fn test_extract_imports_nested_in_try_except() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let file_path = dir.path().join("test.py");
    fs::write(
        &file_path,
        "try:\n    import ujson as json\nexcept ImportError:\n    import json\n",
    )?;

    let imports = extract_imports(&[dir.path().to_path_buf()], &[], false);
    assert!(imports.contains("ujson"));
    assert!(imports.contains("json"));
    Ok(())
}

#[test]
fn test_extract_imports_nested_in_if() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let file_path = dir.path().join("test.py");
    fs::write(
        &file_path,
        "import sys\nif sys.platform == 'win32':\n    import winreg\nelse:\n    import fcntl\n",
    )?;

    let imports = extract_imports(&[dir.path().to_path_buf()], &[], false);
    assert!(imports.contains("sys"));
    assert!(imports.contains("winreg"));
    assert!(imports.contains("fcntl"));
    Ok(())
}

#[test]
fn test_extract_imports_nested_in_class() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let file_path = dir.path().join("test.py");
    fs::write(
        &file_path,
        "class Foo:\n    import dataclasses\n    def method(self):\n        import typing\n",
    )?;

    let imports = extract_imports(&[dir.path().to_path_buf()], &[], false);
    assert!(imports.contains("dataclasses"));
    assert!(imports.contains("typing"));
    Ok(())
}

#[test]
fn test_extract_imports_nested_in_async_blocks() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let file_path = dir.path().join("test.py");
    fs::write(
        &file_path,
        "async def worker(items):\n    import aiohttp\n    async for item in items:\n        import yarl\n    async with aiohttp.ClientSession() as s:\n        import async_timeout\n",
    )?;

    let imports = extract_imports(&[dir.path().to_path_buf()], &[], false);
    assert!(imports.contains("aiohttp"));
    assert!(imports.contains("yarl"));
    assert!(imports.contains("async_timeout"));
    Ok(())
}

#[test]
fn test_extract_imports_nested_in_try_star() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let file_path = dir.path().join("test.py");
    fs::write(
        &file_path,
        "try:\n    import grp_a\nexcept* Exception:\n    import grp_b\nelse:\n    import grp_c\nfinally:\n    import grp_d\n",
    )?;

    let imports = extract_imports(&[dir.path().to_path_buf()], &[], false);
    assert!(imports.contains("grp_a"));
    assert!(imports.contains("grp_b"));
    assert!(imports.contains("grp_c"));
    assert!(imports.contains("grp_d"));
    Ok(())
}
