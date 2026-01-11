//! Tests for finding line numbers on decorated functions and classes.
//! TDD: These tests define the expected behavior - findings should point to
//! the `def`/`class` line, not the decorator line.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::field_reassign_with_default
)]

use cytoscnpy::analyzer::CytoScnPy;
use std::path::Path;

#[test]
fn test_unused_decorated_function_points_to_def_line() {
    // Line 1 is empty, @some_decorator starts at line 2
    let code = r"
@some_decorator
def unused_decorated():
    pass
";

    let mut analyzer = CytoScnPy::default();
    analyzer.confidence_threshold = 0; // Ensure all findings are returned
    let result = analyzer.analyze_code(code, Path::new("decorated_code.py"));

    assert_eq!(
        result.unused_functions.len(),
        1,
        "Expected 1 unused function, got {}",
        result.unused_functions.len()
    );
    let finding = &result.unused_functions[0];

    // The finding should point to line 3 (the `def` line), not line 2 (the decorator)
    // (Line 1 is empty due to leading newline in raw string)
    assert_eq!(
        finding.line, 3,
        "Unused function finding should point to the 'def' line (3), not the decorator line (2)"
    );
}

#[test]
fn test_unused_decorated_class_points_to_class_line() {
    let code = r"
@dataclass
class UnusedClass:
    x: int
";

    let mut analyzer = CytoScnPy::default();
    analyzer.confidence_threshold = 0;
    let result = analyzer.analyze_code(code, Path::new("decorated_code.py"));

    assert_eq!(
        result.unused_classes.len(),
        1,
        "Expected 1 unused class, got {}",
        result.unused_classes.len()
    );
    let finding = &result.unused_classes[0];

    // The finding should point to line 3 (the `class` line), not line 2 (the decorator)
    assert_eq!(
        finding.line, 3,
        "Unused class finding should point to the 'class' line (3), not the decorator line (2)"
    );
}

#[test]
fn test_unused_multi_decorated_function_points_to_def_line() {
    let code = r"
@decorator1
@decorator2
@decorator3
def multi_decorated():
    pass
";

    let mut analyzer = CytoScnPy::default();
    analyzer.confidence_threshold = 0;
    let result = analyzer.analyze_code(code, Path::new("decorated_code.py"));

    assert_eq!(
        result.unused_functions.len(),
        1,
        "Expected 1 unused function, got {}",
        result.unused_functions.len()
    );
    let finding = &result.unused_functions[0];

    // The finding should point to line 5 (the `def` line), not line 2 (the first decorator)
    assert_eq!(
        finding.line, 5,
        "Unused function finding should point to the 'def' line (5), not the first decorator line (2)"
    );
}

#[test]
fn test_quality_finding_on_decorated_function_points_to_def_line() {
    let code = r"
@some_decorator
def complex_function(a, b, c, d, e, f, g):
    if a:
        if b:
            if c:
                return 1
    return 0
";

    let mut analyzer = CytoScnPy::default();
    analyzer.confidence_threshold = 0;
    analyzer.enable_quality = true;
    analyzer.config.cytoscnpy.max_args = Some(5); // Trigger too many args
    let result = analyzer.analyze_code(code, Path::new("decorated_code.py"));

    // Should have at least one quality finding for too many args
    let args_finding = result.quality.iter().find(|f| f.rule_id == "CSP-C303");
    assert!(
        args_finding.is_some(),
        "Should have an ArgumentCount finding. Got findings: {:?}",
        result.quality
    );

    let finding = args_finding.unwrap();
    // The finding should point to line 3 (the `def` line), not line 2 (the decorator)
    assert_eq!(
        finding.line, 3,
        "Quality finding should point to the 'def' line (3), not the decorator line (2)"
    );
}
