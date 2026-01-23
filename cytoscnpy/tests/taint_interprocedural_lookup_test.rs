//! Interprocedural lookup regression tests.

use cytoscnpy::taint::{interprocedural, analyzer::TaintAnalyzer};
use cytoscnpy::utils::LineIndex;
use ruff_python_parser::parse_module;
use std::path::Path;

#[test]
fn test_interprocedural_resolves_module_qualified_functions(
) -> Result<(), Box<dyn std::error::Error>> {
    let source = r"
def vulnerable():
    user = input()
    eval(user)
";

    let parsed = parse_module(source).map_err(|e| format!("Parsing failed: {e:?}"))?;
    let module = parsed.into_syntax();
    let line_index = LineIndex::new(source);
    let analyzer = TaintAnalyzer::default();

    let findings = interprocedural::analyze_module(
        &module.body,
        &analyzer,
        Path::new("sample.py"),
        &line_index,
    );

    assert!(
        findings.iter().any(|finding| finding.rule_id == "CSP-D001"),
        "Expected eval taint finding from interprocedural analysis"
    );

    Ok(())
}
