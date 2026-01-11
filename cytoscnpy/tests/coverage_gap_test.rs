use cytoscnpy::analyzer::CytoScnPy;
use cytoscnpy::halstead::analyze_halstead;
use ruff_python_ast as ast;
use ruff_python_parser::parse_module;
use std::path::PathBuf;

#[test]
fn test_halstead_corpus_coverage() {
    let source = include_str!("coverage_corpus.py");
    let parsed = parse_module(source).expect("Failed to parse corpus");
    let module = parsed.into_syntax();

    // Wrap in Mod::Module
    let ast_mod = ast::Mod::Module(module);

    // Run Halstead analysis
    let metrics = analyze_halstead(&ast_mod);

    assert!(metrics.h1 > 0, "Should have operators");
    assert!(metrics.h2 > 0, "Should have operands");
    assert!(metrics.vocabulary > 0.0);
}

#[test]
fn test_visitor_corpus_coverage() {
    let source = include_str!("coverage_corpus.py");
    let analyzer = CytoScnPy::default();

    // Test analyze_code (uses visitor)
    let results = analyzer.analyze_code(source, &PathBuf::from("coverage_corpus.py"));

    // We expect some findings or at least execution without panic
    // The corpus has some "unused" variables like global G (if unchecked),
    // but main goal is hitting the visitor code.
    assert!(results.analysis_summary.total_lines_analyzed > 0);
}
