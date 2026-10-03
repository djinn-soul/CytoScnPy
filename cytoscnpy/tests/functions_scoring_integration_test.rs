//! Integration tests for function metric aggregation, scoring integration, and CI gates.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::format_push_string)]

use std::fs;
use tempfile::tempdir;

#[test]
fn test_function_stats_calculation_and_scoring_integration() {
    let dir = tempdir().expect("tempdir failed");
    let src = dir.path().join("src");
    fs::create_dir_all(&src).expect("create src dir");

    let py_content = r"
def simple():
    return 1

def complex_and_nested(x, y):
    total = 0
    if x > 0:
        for i in range(x):
            while y > 0:
                if y % 2 == 0:
                    total += i * y
                y -= 1
    return total
";
    fs::write(src.join("math_ops.py"), py_content).expect("write py");

    let roots = vec![dir.path().to_path_buf()];
    let functions = cytoscnpy::functions::analyze_functions(&roots, &[], false);

    assert_eq!(functions.stats.total_functions, 2);
    assert_eq!(functions.stats.max_complexity, 5);
    assert!(functions.stats.avg_complexity >= 2.5);
    assert_eq!(functions.stats.max_nesting, 4);
    assert!(functions.stats.avg_nesting >= 2.0);
    assert!(functions.stats.max_lines >= 9);
    assert!(functions.stats.avg_lines >= 5.0);

    // Run scoring pipeline via score_repository
    let architecture = cytoscnpy::architecture::analyze_architecture(&roots, &[], false);
    let context = cytoscnpy::context::analyze_context(
        &roots,
        &[],
        &cytoscnpy::context::ContextConfig::default(),
    );
    let searchability = cytoscnpy::searchability::analyze_searchability(&roots, &[], false);
    let naming = cytoscnpy::naming::analyze_naming(&roots, &[], false);
    let todos = cytoscnpy::todos::analyze_todos(&roots, &[], false);
    let globals = cytoscnpy::globals::analyze_globals(&roots, &[], false);
    let exceptions = cytoscnpy::exceptions::analyze_exceptions(&roots, &[], false);
    let wildcards = cytoscnpy::wildcards::analyze_wildcards(&roots, &[], false);
    let side_effects = cytoscnpy::side_effects::analyze_side_effects(&roots, &[], false);
    let singletons = cytoscnpy::singletons::analyze_singletons(&roots, &[], false);
    let anti_patterns = cytoscnpy::anti_patterns::analyze_anti_patterns(&roots, &[], false);
    let duplicates = cytoscnpy::duplicates::analyze_duplicates(
        &roots,
        &[],
        &cytoscnpy::duplicates::DuplicatesOptions::default(),
        false,
    );
    let unreferenced = cytoscnpy::unreferenced::analyze_unreferenced(
        &roots,
        &[],
        &cytoscnpy::unreferenced::UnreferencedOptions::default(),
        false,
    );

    let ctx = cytoscnpy::scoring::ScoringContext {
        architecture: &architecture,
        context: &context,
        doctor: None,
        searchability: &searchability,
        naming: &naming,
        todos: &todos,
        globals: &globals,
        exceptions: &exceptions,
        wildcards: &wildcards,
        side_effects: &side_effects,
        singletons: &singletons,
        anti_patterns: &anti_patterns,
        duplicates: &duplicates,
        unreferenced: &unreferenced,
        functions: &functions,
    };

    let score_result =
        cytoscnpy::scoring::score_repository(&ctx, &cytoscnpy::scoring::ScoringOptions::default());
    assert!(score_result.function_stats.is_some());
    let stats = score_result.function_stats.as_ref().unwrap();
    assert_eq!(stats.total_functions, 2);
    assert_eq!(stats.max_complexity, 5);
    assert_eq!(stats.max_nesting, 4);

    // Verify context pressure evidence contains function metrics
    let cp_dim = score_result
        .dimensions
        .iter()
        .find(|d| d.name == "Context pressure")
        .expect("Context pressure dimension");
    assert!(cp_dim.evidence.contains("avg function"));
    assert!(cp_dim.evidence.contains("max nesting depth 4"));

    // Verify terminal output formatting
    let terminal = cytoscnpy::scoring::format_terminal_report(&score_result, false);
    assert!(terminal.contains("Function Metrics:"));
    assert!(terminal.contains("Avg Complexity:"));
    assert!(terminal.contains("Max Nesting: 4"));

    // Verify LLM output formatting
    let llm = cytoscnpy::scoring::format_llm_report(&score_result);
    assert!(llm.contains("- **Function Metrics**: 2 functions"));
    assert!(llm.contains("max nesting depth 4"));
}

#[test]
fn test_context_pressure_and_recommendation_on_extreme_functions() {
    let dir = tempdir().expect("tempdir failed");
    let src = dir.path().join("src");
    fs::create_dir_all(&src).expect("create src dir");

    // Generate a long function with 110 lines and deep nesting
    let mut code = String::from("def extreme_function(a, b, c, d, e, f, g, h, i):\n");
    code.push_str("    x = 0\n");
    code.push_str("    if a:\n        if b:\n            if c:\n                if d:\n                    if e:\n                        if f:\n                            if g:\n                                if h:\n                                    if i:\n                                        x = 42\n");
    for line_idx in 0..100 {
        code.push_str(&format!("    x += {line_idx}\n"));
    }
    code.push_str("    return x\n");

    fs::write(src.join("heavy.py"), code).expect("write py");

    let roots = vec![dir.path().to_path_buf()];
    let functions = cytoscnpy::functions::analyze_functions(&roots, &[], false);
    assert_eq!(functions.stats.total_functions, 1);
    assert!(functions.stats.max_lines > 100);
    assert!(functions.stats.max_nesting >= 9);

    let architecture = cytoscnpy::architecture::analyze_architecture(&roots, &[], false);
    let context = cytoscnpy::context::analyze_context(
        &roots,
        &[],
        &cytoscnpy::context::ContextConfig::default(),
    );
    let searchability = cytoscnpy::searchability::analyze_searchability(&roots, &[], false);
    let naming = cytoscnpy::naming::analyze_naming(&roots, &[], false);
    let todos = cytoscnpy::todos::analyze_todos(&roots, &[], false);
    let globals = cytoscnpy::globals::analyze_globals(&roots, &[], false);
    let exceptions = cytoscnpy::exceptions::analyze_exceptions(&roots, &[], false);
    let wildcards = cytoscnpy::wildcards::analyze_wildcards(&roots, &[], false);
    let side_effects = cytoscnpy::side_effects::analyze_side_effects(&roots, &[], false);
    let singletons = cytoscnpy::singletons::analyze_singletons(&roots, &[], false);
    let anti_patterns = cytoscnpy::anti_patterns::analyze_anti_patterns(&roots, &[], false);
    let duplicates = cytoscnpy::duplicates::analyze_duplicates(
        &roots,
        &[],
        &cytoscnpy::duplicates::DuplicatesOptions::default(),
        false,
    );
    let unreferenced = cytoscnpy::unreferenced::analyze_unreferenced(
        &roots,
        &[],
        &cytoscnpy::unreferenced::UnreferencedOptions::default(),
        false,
    );

    let ctx = cytoscnpy::scoring::ScoringContext {
        architecture: &architecture,
        context: &context,
        doctor: None,
        searchability: &searchability,
        naming: &naming,
        todos: &todos,
        globals: &globals,
        exceptions: &exceptions,
        wildcards: &wildcards,
        side_effects: &side_effects,
        singletons: &singletons,
        anti_patterns: &anti_patterns,
        duplicates: &duplicates,
        unreferenced: &unreferenced,
        functions: &functions,
    };

    let score_result =
        cytoscnpy::scoring::score_repository(&ctx, &cytoscnpy::scoring::ScoringOptions::default());
    let cp_dim = score_result
        .dimensions
        .iter()
        .find(|d| d.name == "Context pressure")
        .expect("Context pressure dimension");
    // Should have rating >= 2 due to max_lines > 100 and max_nesting > 8
    assert!(cp_dim.rating >= 2);

    // Should have recommendation to simplify complex functions
    assert!(score_result
        .recommendations
        .iter()
        .any(|r| r.id == "simplify-complex-functions"));
}

#[test]
fn test_deslop_unified_json_and_function_gates() {
    let dir = tempdir().expect("tempdir failed");
    let src = dir.path().join("src");
    fs::create_dir_all(&src).expect("create src dir");

    let py_content = r"
def nested_func():
    if True:
        if True:
            if True:
                return 42
    return 0
";
    fs::write(src.join("mod.py"), py_content).expect("write py");

    let mut pyproject = String::from("[tool.cytoscnpy.deslop]\n");
    pyproject.push_str("min_health_score = 0\n");
    pyproject.push_str("max_function_lines = 100\n");
    pyproject.push_str("max_function_complexity = 10\n");
    pyproject.push_str("max_nesting_depth = 2\n"); // Will fail since nesting is 3!
    fs::write(dir.path().join("pyproject.toml"), pyproject).expect("write pyproject");

    let roots = vec![dir.path().to_path_buf()];
    let functions = cytoscnpy::functions::analyze_functions(&roots, &[], false);
    assert_eq!(functions.stats.max_nesting, 3);

    let args = vec![
        "deslop".to_owned(),
        dir.path().to_string_lossy().into_owned(),
        "--json".to_owned(),
        "--fail-on-any".to_owned(),
        "--no-git".to_owned(),
    ];
    let mut out = std::io::Cursor::new(Vec::new());
    let code = cytoscnpy::entry_point::run_with_args_to(args, &mut out).expect("run deslop");
    // Should fail gate because max_nesting_depth is 3 > limit 2
    assert_eq!(code, 1);
    let bytes = out.into_inner();
    let parsed: serde_json::Value = serde_json::from_slice(&bytes).expect("parse json");
    assert_eq!(parsed["gates"]["passed"], false);
    let failures = parsed["gates"]["failures"]
        .as_array()
        .expect("failures array");
    assert!(failures.iter().any(|f| f["check"] == "max_nesting_depth"));
    assert_eq!(parsed["functions"]["stats"]["max_nesting"], 3);
}
