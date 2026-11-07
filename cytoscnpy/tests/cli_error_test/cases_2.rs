use super::*;

#[test]
fn test_analysis_paths_with_single_file() {
    let dir = project_tempdir();
    let file_path = dir.path().join("single.py");
    let mut file = File::create(&file_path).unwrap();
    writeln!(file, "def unused(): pass").unwrap();

    let mut analyzer = CytoScnPy::new(
        60,
        false,
        false,
        false,
        false,
        vec![],
        vec![],
        false,
        false,
        Config::default(),
    );

    // Analyze single file instead of directory
    let result = analyzer.analyze_paths(&[file_path]);

    assert_eq!(result.analysis_summary.total_files, 1);
    assert!(!result.unused_functions.is_empty());
}
