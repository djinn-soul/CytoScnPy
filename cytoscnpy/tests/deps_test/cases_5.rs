use super::*;

#[test]
fn test_deps_src_layout_not_flagged_missing() -> anyhow::Result<()> {
    // `src/` layout: the project root has a `src/` directory containing sub-packages.
    // `from src.myapp.models import Foo` should not trigger a missing-dep report for
    // `src` because `src/` is a local namespace package (it contains a sub-package).
    let dir = tempdir()?;
    let root = dir.path();

    fs::write(
        root.join("pyproject.toml"),
        r#"[project]
name = "test-pkg"
version = "0.1.0"
dependencies = []
"#,
    )?;

    // src/ has no __init__.py but contains myapp/ which has Python files.
    let src_dir = root.join("src");
    let myapp_dir = src_dir.join("myapp");
    fs::create_dir_all(&myapp_dir)?;
    fs::write(myapp_dir.join("__init__.py"), "")?;
    fs::write(myapp_dir.join("models.py"), "class Foo: pass\n")?;

    fs::write(root.join("main.py"), "from src.myapp.models import Foo\n")?;

    let (code, output) =
        run_deps_command(vec!["deps".to_owned(), root.to_string_lossy().into_owned()]);

    assert_eq!(code, 0);
    assert!(
        !output.contains("src"),
        "src/ layout directory should not be flagged as missing dependency, got: {output}"
    );
    Ok(())
}
