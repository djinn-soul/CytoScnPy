//! Unit tests for function metric extraction and nesting analysis.
#![allow(clippy::float_cmp)]

use std::path::Path;

use super::extractor::extract_functions;
use super::nesting::calculate_max_nesting;
use super::types::{FunctionInfo, FunctionStats, FunctionsResult};
use ruff_python_parser::parse_module;

#[test]
fn test_direct_calculate_max_nesting() {
    let code = "if a:\n    for b in c:\n        pass\n";
    let parsed = parse_module(code).unwrap();
    let depth = calculate_max_nesting(parsed.suite());
    assert_eq!(depth, 2);
}

#[test]
fn test_simple_function_extraction() {
    let code = r#"
def greet(name: str) -> str:
    """A simple docstring."""
    return f"Hello, {name}!"
"#;
    let functions = extract_functions(code, Path::new("test.py"));
    assert_eq!(functions.len(), 1);
    let f = &functions[0];
    assert_eq!(f.name, "greet");
    assert_eq!(f.simple_name, "greet");
    assert_eq!(f.start_line, 2);
    assert_eq!(f.end_line, 4);
    assert_eq!(f.line_count, 3);
    assert_eq!(f.cyclomatic_complexity, 1);
    assert_eq!(f.max_nesting, 0);
    assert!(!f.is_async);
    assert!(!f.is_method);
    assert!(f.class_name.is_none());
}

#[test]
fn test_async_function_extraction() {
    let code = r"
async def fetch_data(url: str):
    async with aiohttp.ClientSession() as session:
        return await session.get(url)
";
    let functions = extract_functions(code, Path::new("async_test.py"));
    assert_eq!(functions.len(), 1);
    let f = &functions[0];
    assert_eq!(f.name, "fetch_data");
    assert!(f.is_async);
    assert_eq!(f.cyclomatic_complexity, 1);
    assert_eq!(f.max_nesting, 1); // with statement
}

#[test]
fn test_class_method_extraction() {
    let code = r#"
class Service:
    def __init__(self, port: int):
        self.port = port

    def start(self):
        if self.port > 0:
            print("starting")
"#;
    let functions = extract_functions(code, Path::new("service.py"));
    assert_eq!(functions.len(), 2);

    let init = &functions[0];
    assert_eq!(init.name, "Service.__init__");
    assert_eq!(init.simple_name, "__init__");
    assert!(init.is_method);
    assert_eq!(init.class_name.as_deref(), Some("Service"));
    assert_eq!(init.cyclomatic_complexity, 1);
    assert_eq!(init.max_nesting, 0);

    let start = &functions[1];
    assert_eq!(start.name, "Service.start");
    assert_eq!(start.simple_name, "start");
    assert!(start.is_method);
    assert_eq!(start.cyclomatic_complexity, 2); // 1 + if
    assert_eq!(start.max_nesting, 1); // if statement
}

#[test]
fn test_nested_class_method_extraction() {
    let code = r"
class Outer:
    class Inner:
        def process(self):
            pass
";
    let functions = extract_functions(code, Path::new("nested.py"));
    assert_eq!(functions.len(), 1);
    let f = &functions[0];
    assert_eq!(f.name, "Outer.Inner.process");
    assert_eq!(f.simple_name, "process");
    assert!(f.is_method);
    assert_eq!(f.class_name.as_deref(), Some("Outer.Inner"));
}

#[test]
fn test_deep_nesting_calculation() {
    let code = r#"
def deeply_nested(items):
    if items:
        for item in items:
            while item > 0:
                with open("log.txt") as f:
                    try:
                        f.write(str(item))
                    except Exception:
                        pass
                    finally:
                        pass
"#;
    let functions = extract_functions(code, Path::new("deep.py"));
    assert_eq!(functions.len(), 1);
    let f = &functions[0];
    assert_eq!(f.name, "deeply_nested");
    // if -> for -> while -> with -> try = depth 5
    assert_eq!(f.max_nesting, 5);
}

#[test]
fn test_inner_function_independence() {
    let code = r"
def outer():
    if True:
        def inner():
            if False:
                pass
        return inner
";
    let functions = extract_functions(code, Path::new("inner.py"));
    assert_eq!(functions.len(), 2);

    let outer = &functions[0];
    assert_eq!(outer.name, "outer");
    assert_eq!(outer.max_nesting, 1); // Only the outer `if True`

    let inner = &functions[1];
    assert_eq!(inner.name, "inner");
    assert_eq!(inner.max_nesting, 1); // Only the inner `if False`
}

#[test]
fn test_cyclomatic_complexity_branches() {
    let code = r#"
def classify(x, y):
    if x > 0 and y > 0:
        return "Q1"
    elif x < 0 and y > 0:
        return "Q2"
    elif x < 0 and y < 0:
        return "Q3"
    else:
        return "Q4"
"#;
    let functions = extract_functions(code, Path::new("branches.py"));
    assert_eq!(functions.len(), 1);
    let f = &functions[0];
    // Base 1 + 3 branches (if/elifs) + boolean operators (3 `and`)
    assert!(f.cyclomatic_complexity >= 4);
}

#[test]
fn test_function_stats_aggregation() {
    let f1 = FunctionInfo {
        name: "f1".to_owned(),
        simple_name: "f1".to_owned(),
        file: std::path::PathBuf::from("a.py"),
        start_line: 1,
        end_line: 10,
        line_count: 10,
        cyclomatic_complexity: 2,
        max_nesting: 1,
        is_async: false,
        is_method: false,
        class_name: None,
    };
    let f2 = FunctionInfo {
        name: "f2".to_owned(),
        simple_name: "f2".to_owned(),
        file: std::path::PathBuf::from("b.py"),
        start_line: 1,
        end_line: 20,
        line_count: 20,
        cyclomatic_complexity: 6,
        max_nesting: 3,
        is_async: true,
        is_method: false,
        class_name: None,
    };

    let stats = FunctionStats::from_functions(&[f1.clone(), f2.clone()]);
    assert_eq!(stats.total_functions, 2);
    assert_eq!(stats.max_complexity, 6);
    assert_eq!(stats.avg_complexity, 4.0);
    assert_eq!(stats.max_lines, 20);
    assert_eq!(stats.avg_lines, 15.0);
    assert_eq!(stats.max_nesting, 3);
    assert_eq!(stats.avg_nesting, 2.0);

    let result = FunctionsResult {
        functions: vec![f1, f2],
        stats: stats.clone(),
        files_scanned: 2,
    };
    assert!(!result.is_empty());

    let json = serde_json::to_string(&result).expect("Serialize to JSON");
    let deserialized: FunctionsResult = serde_json::from_str(&json).expect("Deserialize JSON");
    assert_eq!(deserialized.stats, stats);
}

#[test]
fn test_empty_function_handling() {
    let functions = extract_functions("", Path::new("empty.py"));
    assert!(functions.is_empty());
    let stats = FunctionStats::from_functions(&functions);
    assert_eq!(stats.total_functions, 0);
    assert_eq!(stats.max_complexity, 0);
    assert_eq!(stats.max_lines, 0);
    assert_eq!(stats.max_nesting, 0);
}
