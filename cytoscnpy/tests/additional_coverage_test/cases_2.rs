use super::*;

#[test]
fn test_exclude_folder_logic() {
    let dir = project_tempdir();

    // Create folder structure
    let src_dir = dir.path().join("src");
    fs::create_dir(&src_dir).unwrap();

    let ignored_dir = dir.path().join("node_modules");
    fs::create_dir(&ignored_dir).unwrap();

    // File in src (should be analyzed)
    let src_file = src_dir.join("main.py");
    let mut f1 = File::create(&src_file).unwrap();
    writeln!(f1, "def my_src_func(): pass").unwrap();

    // File in excluded folder (should NOT be analyzed)
    let ignored_file = ignored_dir.join("ignored.py");
    let mut f2 = File::create(&ignored_file).unwrap();
    writeln!(f2, "def ignored_func(): pass").unwrap();

    let config = Config::default();
    let mut analyzer = CytoScnPy::new(
        50,
        false,
        false,
        false,
        false,
        vec!["node_modules".to_string()], // exclude node_modules
        vec![],
        false,
        false,
        config,
    );

    let result = analyzer.analyze(dir.path());

    // Should find my_src_func from src, but NOT ignored_func from node_modules
    let all_names: Vec<_> = result
        .unused_functions
        .iter()
        .map(|f| f.simple_name.as_str())
        .collect();

    assert!(
        all_names.contains(&"my_src_func"),
        "Should analyze src/main.py. Found: {:?}",
        all_names
    );
    assert!(
        !all_names.contains(&"ignored_func"),
        "Should NOT analyze node_modules/ignored.py"
    );
}

#[test]
fn test_include_folder_overrides_exclude() {
    let dir = project_tempdir();

    // Create a folder that would normally be excluded by default
    let venv_dir = dir.path().join("venv");
    fs::create_dir(&venv_dir).unwrap();

    // File in venv (normally excluded)
    let venv_file = venv_dir.join("special.py");
    let mut f = File::create(&venv_file).unwrap();
    writeln!(f, "def venv_func(): pass").unwrap();

    let config = Config::default();
    let mut analyzer = CytoScnPy::new(
        50,
        false,
        false,
        false,
        false,
        vec![],                   // no extra excludes
        vec!["venv".to_string()], // force include venv
        false,
        false,
        config,
    );

    let result = analyzer.analyze(dir.path());

    // Should find venv_func because venv is force-included
    let all_names: Vec<_> = result
        .unused_functions
        .iter()
        .map(|f| f.simple_name.as_str())
        .collect();

    assert!(
        all_names.contains(&"venv_func"),
        "Should analyze venv when force-included. Found: {:?}",
        all_names
    );
}
