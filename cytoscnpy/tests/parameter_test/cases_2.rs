use super::*;

#[test]
fn test_confidence_threshold_70() {
    let dir = project_tempdir();
    let file_path = dir.path().join("main.py");
    let mut file = File::create(&file_path).unwrap();

    let content = r"
def func(unused_param):
    return 42
";
    write!(file, "{content}").unwrap();

    // Test with confidence threshold 60 (should detect)
    let mut cytoscnpy_60 = CytoScnPy::default().with_confidence(60).with_tests(false);
    let result_60 = cytoscnpy_60.analyze(dir.path());
    assert!(
        !result_60.unused_parameters.is_empty(),
        "Should detect with threshold 60"
    );

    // Test with confidence threshold 80 (should still detect since params have confidence 100)
    let mut cytoscnpy_80 = CytoScnPy::default().with_confidence(80).with_tests(false);
    let result_80 = cytoscnpy_80.analyze(dir.path());
    assert!(
        !result_80.unused_parameters.is_empty(),
        "Should still detect with threshold 80 (params have confidence 100)"
    );
}
