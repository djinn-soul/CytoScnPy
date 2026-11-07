use super::*;

/// Test that `analyze_paths` with a single directory works the same as analyze
#[test]
fn test_analyze_paths_single_directory() {
    let dir = project_tempdir();
    let file_path = dir.path().join("main.py");
    {
        let mut file = File::create(&file_path).unwrap();

        let content = r#"
def used_function():
    return "used"

def unused_function():
    return "unused"

result = used_function()
"#;
        write!(file, "{}", content).unwrap();
    }

    let mut analyzer = CytoScnPy::default().with_confidence(60).with_tests(false);
    let paths = vec![dir.path().to_path_buf()];
    let result = analyzer.analyze_paths(&paths);

    // Should work the same as analyze()
    let unused_funcs: Vec<String> = result
        .unused_functions
        .iter()
        .map(|f| f.simple_name.clone())
        .collect();

    assert!(unused_funcs.contains(&"unused_function".to_owned()));
    assert!(!unused_funcs.contains(&"used_function".to_owned()));
    assert_eq!(result.analysis_summary.total_files, 1);
}

#[test]
fn test_analyze_paths_single_test_helper_respects_include_tests() {
    let dir = project_tempdir();
    let tests_dir = dir.path().join("tests");
    fs::create_dir_all(&tests_dir).unwrap();
    let helper_path = tests_dir.join("helpers.py");
    fs::write(&helper_path, "def unused_test_helper():\n    pass\n").unwrap();

    let mut without_tests = CytoScnPy::default().with_confidence(60).with_tests(false);
    let excluded = without_tests.analyze_paths(std::slice::from_ref(&helper_path));
    assert_eq!(excluded.analysis_summary.total_files, 0);

    let mut with_tests = CytoScnPy::default().with_confidence(60).with_tests(true);
    let included = with_tests.analyze_paths(&[helper_path]);
    assert_eq!(included.analysis_summary.total_files, 1);
}

/// Test that `analyze_paths` with multiple individual files works
#[test]
fn test_analyze_paths_multiple_files() {
    let dir = project_tempdir();

    // Create file1.py
    {
        let file1_path = dir.path().join("file1.py");
        let mut file1 = File::create(&file1_path).unwrap();
        write!(file1, "def unused_in_file1(): pass").unwrap();
    }

    // Create file2.py
    {
        let file2_path = dir.path().join("file2.py");
        let mut file2 = File::create(&file2_path).unwrap();
        write!(file2, "def unused_in_file2(): pass").unwrap();
    }

    // Create file3.py (not included in paths)
    {
        let file3_path = dir.path().join("file3.py");
        let mut file3 = File::create(&file3_path).unwrap();
        write!(file3, "def unused_in_file3(): pass").unwrap();
    }

    let mut analyzer = CytoScnPy::default().with_confidence(60).with_tests(false);

    // Only analyze file1 and file2
    let paths = vec![dir.path().join("file1.py"), dir.path().join("file2.py")];
    let result = analyzer.analyze_paths(&paths);

    let unused_funcs: Vec<String> = result
        .unused_functions
        .iter()
        .map(|f| f.simple_name.clone())
        .collect();

    // Should find unused functions from file1 and file2
    assert!(unused_funcs.contains(&"unused_in_file1".to_owned()));
    assert!(unused_funcs.contains(&"unused_in_file2".to_owned()));

    // Should NOT find unused function from file3 (not in paths)
    assert!(!unused_funcs.contains(&"unused_in_file3".to_owned()));

    // Should only analyze 2 files
    assert_eq!(result.analysis_summary.total_files, 2);
}

/// Test that `analyze_paths` with empty paths analyzes current directory
#[test]
fn test_analyze_paths_empty_defaults_to_current_dir() {
    let dir = project_tempdir();
    let file_path = dir.path().join("main.py");
    {
        let mut file = File::create(&file_path).unwrap();
        write!(file, "def unused_func(): pass").unwrap();
    }

    // Change to temp directory context using RAII guard
    let _guard = cytoscnpy::test_utils::CwdGuard::new(dir.path()).unwrap();

    let mut analyzer = CytoScnPy::default().with_confidence(60).with_tests(false);
    let paths: Vec<PathBuf> = vec![];
    let result = analyzer.analyze_paths(&paths);

    assert_eq!(result.analysis_summary.total_files, 1);
}

/// Test that `analyze_paths` with mixed files and directories works
#[test]
fn test_analyze_paths_mixed_files_and_directories() {
    let dir = project_tempdir();

    // Create a file in root
    let file1_path = dir.path().join("root_file.py");
    {
        let mut file1 = File::create(&file1_path).unwrap();
        write!(file1, "def func_in_root(): pass").unwrap();
    }

    // Create a subdirectory with files
    let subdir = dir.path().join("subdir");
    fs::create_dir(&subdir).unwrap();

    let file2_path = subdir.join("subdir_file.py");
    {
        let mut file2 = File::create(&file2_path).unwrap();
        write!(file2, "def func_in_subdir(): pass").unwrap();
    }

    // Create another file in root (not included)
    let file3_path = dir.path().join("excluded_file.py");
    {
        let mut file3 = File::create(&file3_path).unwrap();
        write!(file3, "def func_excluded(): pass").unwrap();
    }

    let mut analyzer = CytoScnPy::default().with_confidence(60).with_tests(false);

    // Analyze root_file.py and the subdir directory
    let paths = vec![file1_path, subdir];
    let result = analyzer.analyze_paths(&paths);

    let unused_funcs: Vec<String> = result
        .unused_functions
        .iter()
        .map(|f| f.simple_name.clone())
        .collect();

    assert!(unused_funcs.contains(&"func_in_root".to_owned()));
    assert!(unused_funcs.contains(&"func_in_subdir".to_owned()));
    assert!(!unused_funcs.contains(&"func_excluded".to_owned()));
}

/// Test that `analyze_paths` filters non-Python files
#[test]
fn test_analyze_paths_filters_non_python() {
    let dir = project_tempdir();

    // Create a Python file
    let py_path = dir.path().join("script.py");
    {
        let mut py_file = File::create(&py_path).unwrap();
        write!(py_file, "def python_func(): pass").unwrap();
    }

    // Create a non-Python file
    let txt_path = dir.path().join("readme.txt");
    {
        let mut txt_file = File::create(&txt_path).unwrap();
        write!(txt_file, "This is a text file").unwrap();
    }

    // Create a JS file
    let js_path = dir.path().join("script.js");
    {
        let mut js_file = File::create(&js_path).unwrap();
        write!(js_file, "function jsFunc() {{}}").unwrap();
    }

    let mut analyzer = CytoScnPy::default().with_confidence(60).with_tests(false);

    // Pass all files (only .py should be analyzed)
    let paths = vec![py_path, txt_path, js_path];
    let result = analyzer.analyze_paths(&paths);

    // Should only analyze the Python file
    assert_eq!(result.analysis_summary.total_files, 1);

    let unused_funcs: Vec<String> = result
        .unused_functions
        .iter()
        .map(|f| f.simple_name.clone())
        .collect();

    assert!(unused_funcs.contains(&"python_func".to_owned()));
}

/// Test that `analyze_paths` respects exclude_folders
#[test]
fn test_analyze_paths_respects_exclusions() {
    let dir = project_tempdir();

    // Create a .venv directory (should be excluded by default)
    let venv_dir = dir.path().join(".venv");
    fs::create_dir(&venv_dir).unwrap();

    let venv_file = venv_dir.join("venv_script.py");
    {
        let mut venv_f = File::create(&venv_file).unwrap();
        write!(venv_f, "def venv_func(): pass").unwrap();
    }

    // Create a src directory (should be included)
    let src_dir = dir.path().join("src");
    fs::create_dir(&src_dir).unwrap();

    let src_file = src_dir.join("main.py");
    {
        let mut src_f = File::create(&src_file).unwrap();
        write!(src_f, "def src_func(): pass").unwrap();
    }

    let mut analyzer = CytoScnPy::default().with_confidence(60).with_tests(false);

    // Analyze the parent directory (not venv_dir directly)
    // This way the exclusion logic will apply to .venv
    let paths = vec![dir.path().to_path_buf()];
    let result = analyzer.analyze_paths(&paths);

    let unused_funcs: Vec<String> = result
        .unused_functions
        .iter()
        .map(|f| f.simple_name.clone())
        .collect();

    // src_func should be found
    assert!(unused_funcs.contains(&"src_func".to_owned()));

    // venv_func should be excluded (default exclusion)
    assert!(!unused_funcs.contains(&"venv_func".to_owned()));
}

/// Test that `analyze_paths` works with secrets scanning on specific files
#[test]
fn test_analyze_paths_with_secrets_scanning() {
    let dir = project_tempdir();

    // Create a file with a secret
    let file_path = dir.path().join("config.py");
    {
        let mut file = File::create(&file_path).unwrap();
        write!(file, r#"API_KEY = "AKIAIOSFODNN7EXAMPLE""#).unwrap();
    }

    let mut analyzer = CytoScnPy::default()
        .with_confidence(60)
        .with_secrets(true)
        .with_tests(false);

    let paths = vec![file_path];
    let result = analyzer.analyze_paths(&paths);

    // Should find the AWS-style key
    assert!(result.analysis_summary.secrets_count > 0);
}
