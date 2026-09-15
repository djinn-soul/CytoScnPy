//! Unit tests for the `todos` module: types, scanner, and context classification.
#![allow(clippy::unwrap_used)]

use std::path::PathBuf;

use super::scanner::{scan_file_content, scan_files};
use super::types::{TodoKind, TodosResult};

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

fn run(path: &str, content: &str) -> TodosResult {
    let canonical = PathBuf::from(path);
    let skip_ctx = super::scanner::skip_context_sensitive(&canonical);
    let mut matches = Vec::new();
    scan_file_content(&canonical, content, skip_ctx, &mut matches);
    let mut result = TodosResult {
        stats: super::types::TodoStats::default(),
        matches: Vec::new(),
    };
    use std::collections::HashSet;
    let mut affected: HashSet<std::path::PathBuf> = HashSet::new();
    for m in &matches {
        result.stats.increment(m.kind);
        affected.insert(m.file.clone());
    }
    result.stats.affected_files = affected.len();
    result.matches = matches;
    result
}

// ---------------------------------------------------------------------------
// Placeholder detection
// ---------------------------------------------------------------------------

#[test]
fn detects_todo() {
    let r = run("src/engine.py", "# TODO: implement this\nx = 1\n");
    assert_eq!(r.stats.todo_count, 1);
    assert_eq!(r.stats.total, 1);
    assert_eq!(r.matches[0].kind, TodoKind::Todo);
}

#[test]
fn detects_fixme() {
    let r = run("src/parser.py", "# FIXME: broken edge case\n");
    assert_eq!(r.stats.fixme_count, 1);
    assert_eq!(r.matches[0].kind, TodoKind::Fixme);
}

#[test]
fn detects_hack() {
    let r = run(
        "src/utils.py",
        "x = 1  # HACK: workaround for upstream bug\n",
    );
    assert_eq!(r.stats.hack_count, 1);
    assert_eq!(r.matches[0].kind, TodoKind::Hack);
}

#[test]
fn detects_xxx() {
    let r = run("src/model.py", "# XXX: remove before release\n");
    assert_eq!(r.stats.xxx_count, 1);
    assert_eq!(r.matches[0].kind, TodoKind::Xxx);
}

#[test]
fn case_insensitive_todo() {
    let r = run("src/algo.py", "# todo: lowercase also matches\n");
    assert_eq!(r.stats.todo_count, 1);
}

// ---------------------------------------------------------------------------
// Debug print detection
// ---------------------------------------------------------------------------

#[test]
fn detects_debug_print_python() {
    let r = run("src/worker.py", "print('debug value')\nx = 1\n");
    assert_eq!(r.stats.debug_print_count, 1);
    assert_eq!(r.matches[0].kind, TodoKind::DebugPrint);
}

#[test]
fn detects_console_log() {
    let r = run("src/utils.js", "  console.log('test');\n");
    assert_eq!(r.stats.debug_print_count, 1);
}

#[test]
fn skips_debug_print_in_test_file() {
    let r = run("tests/test_utils.py", "print('debug output')\n");
    assert_eq!(
        r.stats.debug_print_count, 0,
        "print in test file should be skipped"
    );
}

#[test]
fn skips_debug_print_in_cli_module() {
    let r = run("src/cli.py", "print('Usage: tool [options]')\n");
    assert_eq!(
        r.stats.debug_print_count, 0,
        "print in cli module should be skipped"
    );
}

#[test]
fn skips_debug_print_in_output_directory() {
    let r = run(
        "src/output/terminal.rs",
        "println!(\"result: {}\", score);\n",
    );
    assert_eq!(r.stats.debug_print_count, 0);
}

#[test]
fn detects_debug_print_in_library_code() {
    let r = run("src/processor.py", "print('debug')\n");
    assert_eq!(r.stats.debug_print_count, 1);
}

// ---------------------------------------------------------------------------
// Commented-out code detection
// ---------------------------------------------------------------------------

#[test]
fn detects_commented_out_if() {
    let r = run("src/engine.py", "# if x > 0:\n#     return True\n");
    // The first line starts with `# if ` → matches
    assert!(r.stats.commented_code_count >= 1);
}

#[test]
fn detects_commented_out_def() {
    let r = run("src/utils.py", "# def old_function():\n");
    assert_eq!(r.stats.commented_code_count, 1);
}

#[test]
fn does_not_flag_normal_comment() {
    let r = run("src/engine.py", "# This is a normal comment\n");
    assert_eq!(r.stats.commented_code_count, 0);
}

// ---------------------------------------------------------------------------
// Clean code
// ---------------------------------------------------------------------------

#[test]
fn clean_code_produces_no_matches() {
    let r = run(
        "src/calculator.py",
        "def add(a, b):\n    return a + b\n\ndef multiply(a, b):\n    return a * b\n",
    );
    assert!(r.is_clean());
    assert_eq!(r.stats.total, 0);
    assert_eq!(r.stats.affected_files, 0);
}

// ---------------------------------------------------------------------------
// Stats aggregation
// ---------------------------------------------------------------------------

#[test]
fn stats_aggregate_correctly() {
    let r = run(
        "src/multi.py",
        "# TODO: fix\n# FIXME: broken\nprint('debug')\n",
    );
    assert_eq!(r.stats.todo_count, 1);
    assert_eq!(r.stats.fixme_count, 1);
    assert_eq!(r.stats.debug_print_count, 1);
    assert_eq!(r.stats.total, 3);
}

// ---------------------------------------------------------------------------
// Multi-file affected_files count via scan_files
// ---------------------------------------------------------------------------

#[test]
fn affected_files_counted_across_multiple_files() {
    let dir = tempfile::TempDir::new().unwrap();
    let a = dir.path().join("a.py");
    let b = dir.path().join("b.py");
    std::fs::write(&a, "# TODO: one\n").unwrap();
    std::fs::write(&b, "# FIXME: two\n").unwrap();
    let result = scan_files(&[a, b]);
    assert_eq!(result.stats.affected_files, 2);
    assert_eq!(result.stats.total, 2);
}

// ---------------------------------------------------------------------------
// False positive avoidance (comment awareness & precision)
// ---------------------------------------------------------------------------

#[test]
fn ignores_code_identifiers_matching_keywords() {
    let r = run(
        "src/logic.py",
        "todo = 1\nfor item in todos:\n    pass\ndef hack(x):\n    return x\nxxx = 42\n",
    );
    assert!(r.is_clean());
    assert_eq!(r.stats.total, 0);
}

#[test]
fn ignores_string_literals_with_keywords() {
    let r = run(
        "src/logic.py",
        "s = \"TODO: not a comment\"\nmsg = '# FIXME: in string'\n",
    );
    assert!(r.is_clean());
    assert_eq!(r.stats.total, 0);
}

#[test]
fn ignores_single_letter_variable_assignment() {
    let r = run("src/logic.py", "p = (1, 2)\np += 3\n");
    assert!(r.is_clean());
    assert_eq!(r.stats.debug_print_count, 0);
}

#[test]
fn ignores_natural_prose_in_comments() {
    let r = run(
        "src/doc.py",
        "# if the request is valid, proceed\n# for backwards compatibility\n# from this moment onwards\n",
    );
    assert!(r.is_clean());
    assert_eq!(r.stats.commented_code_count, 0);
}

#[test]
fn detects_inline_comment_todo() {
    let r = run("src/inline.py", "val = compute()  # TODO: optimize\n");
    assert_eq!(r.stats.todo_count, 1);
    assert_eq!(r.stats.total, 1);
}

#[test]
fn detects_single_line_docstring_todo() {
    let r = run(
        "src/module.py",
        "\"\"\"TODO: implement caching\"\"\"\nx = 1\n",
    );
    assert_eq!(r.stats.todo_count, 1);
    assert_eq!(r.stats.total, 1);
}

#[test]
fn detects_multiline_docstring_todos() {
    let r = run(
        "src/service.py",
        "\"\"\"\nTODO: add retry logic\nFIXME: handle timeout\n\"\"\"\ndef run(): pass\n",
    );
    assert_eq!(r.stats.todo_count, 1);
    assert_eq!(r.stats.fixme_count, 1);
    assert_eq!(r.stats.total, 2);
}

#[test]
fn detects_single_quote_docstring_hack() {
    let r = run("src/helper.py", "'''\nHACK: temporary patch\n'''\n");
    assert_eq!(r.stats.hack_count, 1);
    assert_eq!(r.stats.total, 1);
}

#[test]
fn detects_prefixed_raw_docstring_todo() {
    let r = run(
        "src/parser.py",
        "r\"\"\"TODO: handle regex escape patterns\"\"\"\n",
    );
    assert_eq!(r.stats.todo_count, 1);
    assert_eq!(r.stats.total, 1);
}
