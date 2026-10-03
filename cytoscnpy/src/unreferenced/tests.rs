use super::analyzer::analyze_unreferenced_files;
use super::heuristics::{contains_as_word, should_skip_function, UnreferencedOptions};
use super::reporter::{print_json_report, print_terminal_report};
use super::types::{UnreferencedFunction, UnreferencedResult};
use std::fs;
use tempfile::tempdir;

#[test]
fn test_contains_as_word() {
    assert!(contains_as_word("foo = 1", "foo"));
    assert!(contains_as_word("def foo(x): pass", "foo"));
    assert!(!contains_as_word("foobar = 1", "foo"));
    assert!(!contains_as_word("bar_foo_baz = 1", "foo"));
    assert!(contains_as_word("result = func(foo)", "foo"));
}

#[test]
fn test_should_skip_function() {
    let opts = UnreferencedOptions::default();

    // Skip short names
    assert!(should_skip_function("calc", 20, &opts));
    // Skip small functions
    assert!(should_skip_function("calculate_tax_rate", 10, &opts));
    // Skip test functions
    assert!(should_skip_function("test_user_registration", 20, &opts));
    assert!(should_skip_function("TestClassRunner", 20, &opts));
    // Skip dunder methods
    assert!(should_skip_function("__init__", 30, &opts));
    // Skip framework prefixes
    assert!(should_skip_function("get_user_by_email", 25, &opts));
    assert!(should_skip_function("handle_payment_webhook", 25, &opts));
    assert!(should_skip_function("render_account_dashboard", 25, &opts));

    // Keep legitimate business domain function
    assert!(!should_skip_function(
        "calculate_customer_discount_ratio",
        20,
        &opts
    ));
}

#[test]
fn test_result_is_clean() {
    let mut res = UnreferencedResult::default();
    assert!(res.is_clean());

    res.items.push(UnreferencedFunction {
        file: "isolated.py".into(),
        name: "calculate_customer_discount_ratio".to_owned(),
        start_line: 1,
        end_line: 25,
        line_count: 25,
        node_kind: "function".to_owned(),
        class_name: None,
    });
    assert!(!res.is_clean());
}

#[test]
fn test_analyzer_ignores_connected_files() {
    let dir = tempdir().unwrap();
    let file_a = dir.path().join("main.py");
    let file_b = dir.path().join("billing.py");

    // billing.py defines a large function
    let mut billing_code =
        String::from("def calculate_customer_discount_ratio(customer, purchases):\n");
    for _ in 0..20 {
        billing_code.push_str("    line_val = customer.points * 42\n");
    }
    billing_code.push_str("    return line_val\n");
    fs::write(&file_b, &billing_code).unwrap();

    // main.py imports billing
    fs::write(&file_a, "import billing\n\nprint('starting app')\n").unwrap();

    let opts = UnreferencedOptions {
        min_lines: 15,
        min_name_len: 8,
        include_tests: false,
    };

    let res = analyze_unreferenced_files(&[file_a, file_b], &opts);
    // billing is connected via import from main, so it is NOT isolated
    assert_eq!(res.items.len(), 0);
    assert_eq!(res.stats.total_unreferenced_functions, 0);
}

#[test]
fn test_analyzer_detects_unreferenced_in_isolated_file() {
    let dir = tempdir().unwrap();
    let main_py = dir.path().join("main.py");
    let orphan_py = dir.path().join("orphan_utils.py");

    fs::write(&main_py, "def start():\n    print('hello')\n").unwrap();

    let mut orphan_code = String::from("def compute_abandoned_metric_score(dataset, weights):\n");
    for _ in 0..20 {
        orphan_code.push_str("    val = dataset.read(0) * weights.get(0, 1)\n");
    }
    orphan_code.push_str("    return val\n");
    fs::write(&orphan_py, &orphan_code).unwrap();

    let opts = UnreferencedOptions {
        min_lines: 15,
        min_name_len: 8,
        include_tests: false,
    };

    let res = analyze_unreferenced_files(&[main_py, orphan_py.clone()], &opts);
    assert_eq!(res.items.len(), 1);
    assert_eq!(res.items[0].name, "compute_abandoned_metric_score");
    assert_eq!(res.items[0].file, orphan_py);
    assert_eq!(res.stats.isolated_files_count, 1);

    // Verify reporters
    let mut term = Vec::new();
    print_terminal_report(&res, Some(dir.path()), &mut term).unwrap();
    let term_str = String::from_utf8(term).unwrap();
    assert!(term_str.contains("Unreferenced Large Functions"));
    assert!(term_str.contains("compute_abandoned_metric_score"));

    let mut json_out = Vec::new();
    print_json_report(&res, &mut json_out).unwrap();
    let json_str = String::from_utf8(json_out).unwrap();
    assert!(json_str.contains("\"total_unreferenced_functions\": 1"));
}
