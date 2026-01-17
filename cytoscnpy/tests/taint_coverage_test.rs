//! Taint analysis coverage tests.
use cytoscnpy::taint::analyzer::{TaintAnalyzer, TaintConfig, TaintSourcePlugin};
use cytoscnpy::taint::call_graph::CallGraph;
use cytoscnpy::taint::sources::check_taint_source;
use cytoscnpy::taint::TaintInfo;
use cytoscnpy::utils::LineIndex;
use ruff_python_ast::Expr;
use ruff_python_parser::{parse_expression, parse_module};
use std::path::PathBuf;

struct DummySourcePlugin;
impl TaintSourcePlugin for DummySourcePlugin {
    fn name(&self) -> &'static str {
        "Dummy"
    }
    fn check_source(&self, _expr: &Expr, _line_index: &LineIndex) -> Option<TaintInfo> {
        None
    }
}

#[test]
fn test_attr_checks_coverage() -> Result<(), Box<dyn std::error::Error>> {
    let expressions = vec![
        ("sys.argv", true),
        ("os.environ", true),
        ("os.environ.get('x')", true),
        ("os.getenv('x')", true),
        ("request.GET", true),
        ("request.POST", true),
        ("request.body", true),
        ("request.COOKIES", true),
        ("request.args", true),
        ("request.form", true),
        ("request.data", true),
        ("request.json", true),
        ("request.cookies", true),
        ("request.files", true),
        ("request.values", true),
        ("request.args.get", true),
    ];

    for (expr_str, should_match) in expressions {
        let parsed =
            parse_expression(expr_str).map_err(|e| format!("Failed to parse {expr_str}: {e:?}"))?;
        let expr = parsed.into_syntax();
        let body = expr.body;
        let line_index = LineIndex::new(expr_str);
        let result = check_taint_source(&body, &line_index);
        if should_match {
            assert!(result.is_some(), "Expected match for {expr_str}");
        } else {
            assert!(result.is_none(), "Expected no match for {expr_str}");
        }
    }
    Ok(())
}

#[test]
fn test_call_graph_coverage() -> Result<(), Box<dyn std::error::Error>> {
    let source = include_str!("taint_corpus.py");
    let parsed = parse_module(source).map_err(|e| format!("Failed to parse module: {e:?}"))?;
    let module = parsed.into_syntax();

    let mut cg = CallGraph::new();
    cg.build_from_module(&module.body);

    // Check nodes exist
    assert!(cg.nodes.contains_key("a"));
    assert!(cg.nodes.contains_key("process_data"));

    // Check edges
    let node = cg.nodes.get("a").ok_or("Node 'a' not found")?;
    assert!(node.calls.contains("process_data"));
    Ok(())
}

#[test]
fn test_taint_analyzer_full_corpus() {
    let source = include_str!("taint_corpus.py");
    let config = TaintConfig::all_levels();
    let analyzer = TaintAnalyzer::new(config);

    let path = PathBuf::from("taint_corpus.py");
    let findings = analyzer.analyze_file(source, &path);

    // Module-level flows like eval(input()) should be found
    assert!(!findings.is_empty(), "Expected taint findings in corpus");
}

#[test]
fn test_taint_analyzer_project() {
    let source = include_str!("taint_corpus.py");
    let mut analyzer = TaintAnalyzer::new(TaintConfig::all_levels());
    let files = vec![(PathBuf::from("taint_corpus.py"), source.to_owned())];

    let findings = analyzer.analyze_project(&files);
    assert!(
        !findings.is_empty(),
        "Expected findings in project analysis"
    );
}

#[test]
fn test_plugin_registration() {
    let config = TaintConfig::default();
    let mut analyzer = TaintAnalyzer::empty(config);
    analyzer.add_source(DummySourcePlugin);

    assert_eq!(analyzer.plugins.sources.len(), 1);
    assert_eq!(analyzer.plugins.sources[0].name(), "Dummy");
}

#[test]
fn test_taint_analyzer_no_intra() {
    let source = include_str!("taint_corpus.py");
    let mut config = TaintConfig::all_levels();
    config.intraprocedural = false;
    let analyzer = TaintAnalyzer::new(config);

    let path = PathBuf::from("taint_corpus.py");
    let _findings = analyzer.analyze_file(source, &path);
}
