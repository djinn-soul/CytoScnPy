use super::*;

#[test]
fn test_print_report_quiet() {
    let mut buffer = Vec::new();
    let result = create_mock_result();
    let res = print_report_quiet(&mut buffer, &result);
    assert!(res.is_ok());
    let output = String::from_utf8(buffer).unwrap();
    assert!(output.contains("SUMMARY"));
    // Should not contain detailed tables
    assert!(!output.contains("Unreachable Functions"));
}
