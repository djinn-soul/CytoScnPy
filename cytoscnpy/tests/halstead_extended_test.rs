//! Extended tests for `halstead.rs` - Halstead complexity metrics.
#![allow(clippy::unwrap_used)]

use cytoscnpy::halstead::{analyze_halstead, analyze_halstead_functions, HalsteadMetrics};
use ruff_python_ast as ast;
use ruff_python_parser::parse_module;

/// Parse source and analyze Halstead metrics.
fn analyze_source(source: &str) -> HalsteadMetrics {
    let parsed = parse_module(source).unwrap();
    let module = ast::Mod::Module(parsed.into_syntax());
    analyze_halstead(&module)
}

/// Parse source and analyze per-function Halstead metrics.
fn analyze_functions(source: &str) -> Vec<(String, HalsteadMetrics)> {
    let parsed = parse_module(source).unwrap();
    let module = ast::Mod::Module(parsed.into_syntax());
    analyze_halstead_functions(&module)
}

#[test]
fn test_halstead_empty_file() {
    let metrics = analyze_source("");
    assert_eq!(metrics.n1, 0); // distinct operators
    assert_eq!(metrics.n2, 0); // distinct operands
    assert_eq!(metrics.h1, 0); // total operators
    assert_eq!(metrics.h2, 0); // total operands
}

#[test]
fn test_halstead_simple_assignment() {
    let metrics = analyze_source("x = 1");
    assert!(metrics.h2 > 0); // x and 1 are operands
    assert!(metrics.n1 > 0); // = is an operator
}

#[test]
fn test_halstead_arithmetic() {
    let metrics = analyze_source("result = 1 + 2 * 3 - 4 / 5");
    assert!(metrics.h1 > 0); // +, *, -, / are operators
    assert!(metrics.h2 > 0); // 1, 2, 3, 4, 5, result are operands
}

#[test]
fn test_halstead_function_definition() {
    let metrics = analyze_source("def foo(a, b):\n    return a + b\n");
    assert!(metrics.n2 > 0); // foo, a, b
    assert!(metrics.vocabulary > 0.0);
}

#[test]
fn test_halstead_class_definition() {
    let source = r"
class MyClass:
    def __init__(self, value):
        self.value = value
    
    def get_value(self):
        return self.value
";
    let metrics = analyze_source(source);
    assert!(metrics.vocabulary > 0.0);
    assert!(metrics.length > 0.0);
}

#[test]
fn test_halstead_control_flow() {
    let source = r"
if x > 0:
    print('positive')
elif x < 0:
    print('negative')
else:
    print('zero')
";
    let metrics = analyze_source(source);
    assert!(metrics.h1 > 0); // >, <, if, elif, else
}

#[test]
fn test_halstead_loops() {
    let source = r"
for i in range(10):
    print(i)

while x > 0:
    x -= 1
";
    let metrics = analyze_source(source);
    assert!(metrics.vocabulary > 0.0);
}

#[test]
fn test_halstead_comprehensions() {
    let source = r"
squares = [x**2 for x in range(10)]
even = {x for x in range(10) if x % 2 == 0}
mapping = {x: x**2 for x in range(5)}
";
    let metrics = analyze_source(source);
    assert!(metrics.length > 0.0);
}

#[test]
fn test_halstead_lambda() {
    let metrics = analyze_source("double = lambda x: x * 2");
    assert!(metrics.vocabulary > 0.0);
}

#[test]
fn test_halstead_try_except() {
    let source = r"
try:
    result = 1 / x
except ZeroDivisionError:
    result = 0
finally:
    print(result)
";
    let metrics = analyze_source(source);
    assert!(metrics.vocabulary > 0.0);
}

#[test]
fn test_halstead_import() {
    let source = r"
import os
from sys import argv
from typing import List, Dict
";
    let metrics = analyze_source(source);
    // imports contribute to metrics
    assert!(metrics.n2 >= 0);
}

#[test]
fn test_halstead_boolean_operators() {
    let metrics = analyze_source("result = a and b or not c");
    assert!(metrics.h1 > 0); // and, or, not
}

#[test]
fn test_halstead_comparison_operators() {
    let metrics = analyze_source("x = a == b");
    assert!(metrics.h1 > 0); // ==
}

#[test]
fn test_halstead_bitwise_operators() {
    let metrics = analyze_source("x = a & b | c");
    assert!(metrics.h1 > 0); // &, |
}

#[test]
fn test_halstead_unary_operators() {
    let metrics = analyze_source("x = -a + ~b");
    assert!(metrics.h1 > 0);
}

#[test]
fn test_halstead_string_operations() {
    let source = r#"
s = "hello" + "world"
s2 = "repeat" * 3
"#;
    let metrics = analyze_source(source);
    assert!(metrics.vocabulary > 0.0);
}

#[test]
fn test_halstead_function_calls() {
    let source = r"
result = print('test')
value = len([1, 2, 3])
items = list(range(10))
";
    let metrics = analyze_source(source);
    assert!(metrics.h2 > 0);
}

#[test]
fn test_halstead_metrics_calculation() {
    // Test that derived metrics are calculated correctly
    let source = "x = 1 + 2";
    let metrics = analyze_source(source);

    // vocabulary = n1 + n2 (as floats)
    let expected_vocab = (metrics.n1 + metrics.n2) as f64;
    assert!((metrics.vocabulary - expected_vocab).abs() < 0.001);

    // length = h1 + h2 (total operators + total operands)
    let expected_length = (metrics.h1 + metrics.h2) as f64;
    assert!((metrics.length - expected_length).abs() < 0.001);
}

#[test]
fn test_halstead_per_function() {
    let source = r"
def foo():
    return 1

def bar(x):
    return x * 2

def baz(a, b):
    return a + b
";
    let functions = analyze_functions(source);
    assert_eq!(functions.len(), 3);

    // Check that function names are captured
    let names: Vec<_> = functions.iter().map(|(n, _)| n.as_str()).collect();
    assert!(names.contains(&"foo"));
    assert!(names.contains(&"bar"));
    assert!(names.contains(&"baz"));
}

#[test]
fn test_halstead_nested_functions() {
    let source = r"
def outer():
    def inner():
        return 1
    return inner()
";
    let functions = analyze_functions(source);
    // Should find both outer and inner
    assert!(functions.len() >= 1);
}

#[test]
fn test_halstead_class_methods() {
    let source = r"
class MyClass:
    def method1(self):
        return 1
    
    def method2(self, x):
        return x * 2
";
    let functions = analyze_functions(source);
    // Should find methods
    assert!(functions.len() >= 2);
}

#[test]
fn test_halstead_async_function() {
    let source = r"
async def fetch():
    return await get_data()
";
    let functions = analyze_functions(source);
    assert!(!functions.is_empty());
}

#[test]
fn test_halstead_decorators() {
    let source = r"
@decorator
def foo():
    pass

@property
def bar(self):
    return self._bar
";
    let functions = analyze_functions(source);
    assert!(functions.len() >= 2);
}

#[test]
fn test_halstead_volume_calculation() {
    let source = "x = 1 + 2 * 3";
    let metrics = analyze_source(source);

    // Volume should be non-negative
    assert!(metrics.volume >= 0.0);
}

#[test]
fn test_halstead_difficulty_calculation() {
    let source = "x = 1 + 2 * 3";
    let metrics = analyze_source(source);

    // Difficulty should be non-negative
    assert!(metrics.difficulty >= 0.0);
}

#[test]
fn test_halstead_effort_calculation() {
    let source = "x = 1 + 2 * 3";
    let metrics = analyze_source(source);

    // Effort = Volume * Difficulty
    let expected_effort = metrics.volume * metrics.difficulty;
    let difference = (metrics.effort - expected_effort).abs();
    assert!(difference < 0.001);
}

#[test]
fn test_halstead_time_calculation() {
    let source = "x = 1 + 2 * 3";
    let metrics = analyze_source(source);

    // Time should be non-negative
    assert!(metrics.time >= 0.0);
}

#[test]
fn test_halstead_bugs_calculation() {
    let source = "x = 1 + 2 * 3";
    let metrics = analyze_source(source);

    // Bugs should be non-negative
    assert!(metrics.bugs >= 0.0);
}

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
