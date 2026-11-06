//! Every fix suggestion an editor can apply must leave valid Python.

#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use cytoscnpy::analyzer::types::FixEdit;
use cytoscnpy::analyzer::{AnalysisResult, CytoScnPy};
use std::path::PathBuf;

const CASES: &[(&str, &str)] = &[
    ("import_first_of_two", "import os, sys\nprint(sys)\n"),
    ("import_last_of_two", "import os, sys\nprint(os)\n"),
    ("import_middle", "import a, b, c\nprint(a, c)\n"),
    ("import_all_aliases", "import os, sys\n"),
    ("from_import_partial", "from typing import List, Dict\nx: Dict = {}\nprint(x)\n"),
    ("from_import_all", "from typing import List, Dict\n"),
    (
        "from_import_paren_multiline",
        "from typing import (\n    List,\n    Dict,\n    Any,\n)\nprint(Dict)\n",
    ),
    ("import_as", "import numpy as np, os as o\nprint(o)\n"),
    ("semicolon_imports", "import os; import sys\nprint(1)\n"),
    ("semicolon_mixed", "import os; x = 1\nprint(x)\n"),
    ("import_only_in_if", "if True:\n    import os\n"),
    ("import_in_func_only", "def f():\n    import os\n\nf()\n"),
    ("variable_simple", "def f():\n    x = 1\n    return 2\nf()\n"),
    ("variable_annassign", "def f():\n    x: int = 1\n    return 2\nf()\n"),
    (
        "variable_with_underscore_used",
        "def f():\n    _ = 5\n    x = 1\n    return _\nf()\n",
    ),
    ("variable_for", "def f():\n    for i in range(3):\n        pass\nf()\n"),
    ("variable_walrus", "def f():\n    if (n := 3):\n        return 1\nf()\n"),
    ("func_only_in_if", "if True:\n    def only():\n        pass\n"),
    (
        "two_funcs_only_in_if",
        "if True:\n    def a():\n        pass\n    def b():\n        pass\n",
    ),
    ("func_only_in_func", "def outer():\n    def inner():\n        pass\nouter()\n"),
    (
        "class_methods_all_unused",
        "class Box:\n    def m1(self):\n        pass\n    def m2(self):\n        pass\n\nBox()\n",
    ),
    ("class_single_method", "class Box:\n    def m1(self):\n        pass\n\nBox()\n"),
    ("class_and_its_method", "class Dead:\n    def m(self):\n        pass\n"),
    (
        "unicode",
        "import os, sys  # ünïcødé\ndef f():\n    s = 'héllo 🐍'\n    return 1\nf()\nprint(sys)\n",
    ),
    (
        "crlf",
        "import os, sys\r\ndef f():\r\n    x = 1\r\n    return 2\r\nf()\r\nprint(sys)\r\n",
    ),
    ("one_line_suite", "def f(): x = 1; return 2\nf()\n"),
    (
        "match_case",
        "def f(v):\n    match v:\n        case 1:\n            def g():\n                pass\n    return 1\nf(1)\n",
    ),
    (
        "everything",
        "import os, sys\nfrom typing import List, Dict\n\ndef used(a, b):\n    x = 1\n    return a\n\n\
         class Box:\n    def m1(self):\n        pass\n    def m2(self):\n        pass\n\n\
         if True:\n    def only():\n        pass\n\n\
         def main():\n    print(used(1, 2), sys, Dict, Box())\n",
    ),
];

fn analyze(code: &str) -> AnalysisResult {
    let mut result = CytoScnPy::default()
        .with_confidence(80)
        .analyze_code(code, &PathBuf::from("m.py"));
    cytoscnpy::commands::verify_fix_suggestions_with(&mut result, |_| Some(code.to_owned()));
    result
}

fn apply(code: &str, edits: &[FixEdit]) -> String {
    let mut sorted = edits.to_vec();
    sorted.sort_by_key(|edit| std::cmp::Reverse(edit.start_byte));
    let mut fixed = code.to_owned();
    for edit in sorted {
        fixed.replace_range(edit.start_byte..edit.end_byte, &edit.replacement);
    }
    fixed
}

fn assert_parses(case: &str, label: &str, fixed: &str) {
    if let Err(error) = ruff_python_parser::parse_module(fixed) {
        panic!("{case}: {label} produced invalid Python ({error}):\n{fixed}");
    }
}

#[test]
fn every_suggested_fix_leaves_valid_python() {
    for (case, code) in CASES {
        let result = analyze(code);
        let fixable: Vec<_> = [
            &result.unused_functions,
            &result.unused_methods,
            &result.unused_classes,
            &result.unused_imports,
            &result.unused_variables,
        ]
        .into_iter()
        .flatten()
        .filter_map(|def| def.fix.as_deref().map(|fix| (def, fix)))
        .collect();
        assert!(
            !fixable.is_empty(),
            "{case}: expected at least one fixable finding"
        );

        for (def, fix) in &fixable {
            let single = FixEdit {
                end_byte: fix.end_byte,
                replacement: fix.replacement.clone(),
                start_byte: fix.start_byte,
            };
            assert_parses(case, &def.simple_name, &apply(code, &[single]));
            if !fix.rule_fix_all.is_empty() {
                assert_parses(case, "rule_fix_all", &apply(code, &fix.rule_fix_all));
            }
            if !fix.file_fix_all.is_empty() {
                assert_parses(case, "file_fix_all", &apply(code, &fix.file_fix_all));
            }
        }
        if fixable.len() >= 2 {
            assert!(
                fixable.iter().any(|(_, fix)| !fix.file_fix_all.is_empty()),
                "{case}: several fixable findings need a combined plan"
            );
        }
    }
}

#[test]
fn unused_parameters_get_no_fix() {
    let result = analyze("def f(a, b):\n    return a\nf(1, 2)\n");
    assert!(!result.unused_parameters.is_empty());
    assert!(result.unused_parameters.iter().all(|def| def.fix.is_none()));
}

#[test]
fn combined_fix_keeps_a_class_body_when_all_methods_go() {
    let code =
        "class Box:\n    def m1(self):\n        pass\n    def m2(self):\n        pass\n\nBox()\n";
    let result = analyze(code);
    let plan = result
        .unused_methods
        .iter()
        .find_map(|def| {
            def.fix
                .as_deref()
                .filter(|fix| !fix.rule_fix_all.is_empty())
                .map(|fix| fix.rule_fix_all.clone())
        })
        .expect("two unused methods should carry a combined plan");
    let fixed = apply(code, &plan);
    assert_parses("class_methods_all_unused", "rule_fix_all", &fixed);
    assert!(fixed.contains("class Box:\n    pass"), "{fixed}");
}
