use super::*;

#[test]
fn test_halstead_calculated_length() {
    let source = "x = 1 + 2";
    let metrics = analyze_source(source);

    // calculated_length should be non-negative
    assert!(metrics.calculated_length >= 0.0);
}

#[test]
fn test_halstead_default() {
    let metrics = HalsteadMetrics::default();
    assert_eq!(metrics.h1, 0);
    assert_eq!(metrics.h2, 0);
    assert_eq!(metrics.n1, 0);
    assert_eq!(metrics.n2, 0);
    assert!((metrics.vocabulary - 0.0).abs() < 0.001);
}
