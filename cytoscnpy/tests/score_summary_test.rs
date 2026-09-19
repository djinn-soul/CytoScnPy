//! Tests for repository summary, language breakdown, test/source counts, and configuration inventory in score results.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use tempfile::TempDir;

use cytoscnpy::scoring::{score_repository, ScoringContext, ScoringOptions};

#[test]
fn test_score_includes_repository_summary_and_languages() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    // Create a dummy python file and test file
    let src_dir = root.join("src");
    let tests_dir = root.join("tests");
    fs::create_dir_all(&src_dir).unwrap();
    fs::create_dir_all(&tests_dir).unwrap();

    fs::write(
        src_dir.join("lib.py"),
        "def compute(x: int) -> int:\n    return x * 2\n",
    )
    .unwrap();
    fs::write(
        tests_dir.join("test_lib.py"),
        "from src.lib import compute\n\ndef test_compute():\n    assert compute(2) == 4\n",
    )
    .unwrap();

    // Create pyproject.toml
    fs::write(
        root.join("pyproject.toml"),
        "[tool.pytest.ini_options]\nminversion = \"6.0\"\n",
    )
    .unwrap();

    let doctor = cytoscnpy::doctor::run_doctor(
        root,
        &cytoscnpy::doctor::DoctorConfig {
            fail_on_missing: false,
            verbose: false,
            excludes: vec![],
        },
    );

    let architecture =
        cytoscnpy::architecture::analyze_architecture(&[root.to_path_buf()], &[], false);
    let context = cytoscnpy::context::analyze_context(
        &[root.to_path_buf()],
        &[],
        &cytoscnpy::context::ContextConfig::default(),
    );
    let searchability =
        cytoscnpy::searchability::analyze_searchability(&[root.to_path_buf()], &[], false);
    let naming = cytoscnpy::naming::analyze_naming(&[root.to_path_buf()], &[], false);
    let todos = cytoscnpy::todos::analyze_todos(&[root.to_path_buf()], &[], false);
    let globals = cytoscnpy::globals::analyze_globals(&[root.to_path_buf()], &[], false);
    let exceptions = cytoscnpy::exceptions::analyze_exceptions(&[root.to_path_buf()], &[], false);
    let wildcards = cytoscnpy::wildcards::analyze_wildcards(&[root.to_path_buf()], &[], false);
    let side_effects =
        cytoscnpy::side_effects::analyze_side_effects(&[root.to_path_buf()], &[], false);
    let singletons = cytoscnpy::singletons::analyze_singletons(&[root.to_path_buf()], &[], false);
    let anti_patterns =
        cytoscnpy::anti_patterns::analyze_anti_patterns(&[root.to_path_buf()], &[], false);
    let duplicates = cytoscnpy::duplicates::analyze_duplicates(
        &[root.to_path_buf()],
        &[],
        &cytoscnpy::duplicates::DuplicatesOptions::default(),
        false,
    );
    let unreferenced = cytoscnpy::unreferenced::analyze_unreferenced(
        &[root.to_path_buf()],
        &[],
        &cytoscnpy::unreferenced::UnreferencedOptions::default(),
        false,
    );

    let ctx = ScoringContext {
        architecture: &architecture,
        context: &context,
        doctor: Some(&doctor),
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
    };

    let result = score_repository(&ctx, &ScoringOptions::default());
    assert!(result.summary.is_some());
    let summary = result.summary.as_ref().unwrap();

    assert!(summary.total_files >= 2);
    assert_eq!(summary.test_files, 1);
    assert!(summary.source_files >= 1);
    assert!(summary.test_to_source_ratio > 0.0);
    assert!(summary.languages.iter().any(|l| l.name == "Python"));

    // Verify terminal output formatting contains summary block
    let terminal = cytoscnpy::scoring::format_terminal_report(&result, false);
    assert!(terminal.contains("Codebase Summary:"));
    assert!(terminal.contains("Source Files:"));
    assert!(terminal.contains("Test Files:"));
    assert!(terminal.contains("Python"));

    // Verify LLM output formatting contains summary block
    let llm = cytoscnpy::scoring::format_llm_report(&result);
    assert!(llm.contains("- **Codebase Totals**:"));
    assert!(llm.contains("- **Languages**:"));

    // Verify JSON serialization includes summary object
    let json_val = serde_json::to_value(&result).unwrap();
    assert!(json_val.get("summary").is_some());
    assert_eq!(json_val["summary"]["test_files"], 1);
}
