//! Scanner for TODO/FIXME/HACK/XXX annotations, debug prints, and commented-out code.

use rayon::prelude::*;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::commands::utils::find_python_files;

use super::types::{TodoKind, TodoMatch, TodoStats, TodosResult};

const OUTPUT_STEMS: &[&str] = &["main", "cli", "cmd", "output", "console", "render"];

const OUTPUT_PARENT_DIRS: &[&str] = &["output", "cli", "console", "render"];

const TEST_PARENT_DIRS: &[&str] = &["test", "tests", "spec", "specs", "__tests__"];

fn parent_dir_matches(path: &Path, dirs: &[&str]) -> bool {
    path.parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .is_some_and(|dir| dirs.iter().any(|&d| dir.eq_ignore_ascii_case(d)))
}

/// Returns `true` if the file is likely a test file by name or direct parent directory.
fn is_test_context(path: &Path) -> bool {
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    stem.starts_with("test_")
        || stem.ends_with("_test")
        || stem.ends_with("_spec")
        || stem.eq_ignore_ascii_case("conftest")
        || parent_dir_matches(path, TEST_PARENT_DIRS)
}

/// Returns `true` when debug-print detection should be skipped for this file.
pub(crate) fn skip_context_sensitive(path: &Path) -> bool {
    if is_test_context(path) {
        return true;
    }
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    OUTPUT_STEMS.iter().any(|&s| stem.eq_ignore_ascii_case(s))
        || parent_dir_matches(path, OUTPUT_PARENT_DIRS)
}

const MAX_SNIPPET: usize = 120;

fn truncate_snippet(s: &str) -> &str {
    s.char_indices()
        .nth(MAX_SNIPPET)
        .map_or(s, |(i, _)| &s[..i])
}

/// Extracts the comment portion of a source line, or returns `None` if the line
/// does not contain a comment. Ignores comment markers inside string literals.
fn extract_comment(line: &str) -> Option<&str> {
    let t = line.trim_start();
    if let Some(s) = t
        .strip_prefix("//")
        .or_else(|| t.strip_prefix('#'))
        .or_else(|| t.strip_prefix("/*"))
    {
        return Some(s);
    }
    if t.starts_with('*') && !t.starts_with("**") {
        return Some(&t[1..]);
    }

    let (mut in_q, mut prev) = (None, '\0');
    for (i, c) in line.char_indices() {
        if (c == '\'' || c == '"') && prev != '\\' {
            in_q = if in_q == Some(c) {
                None
            } else if in_q.is_none() {
                Some(c)
            } else {
                in_q
            };
        } else if in_q.is_none() {
            if c == '#' {
                return Some(&line[i + 1..]);
            }
            if line[i..].starts_with("//") {
                return Some(&line[i + 2..]);
            }
        }
        prev = c;
    }
    None
}

/// Checks whether `line` contains a commented-out code block rather than prose.
fn is_commented_code_line(line: &str) -> bool {
    let Some(body) = extract_comment(line).map(str::trim_start) else {
        return false;
    };
    if body.is_empty() {
        return false;
    }
    if ["def ", "class ", "fn ", "func "]
        .iter()
        .any(|k| body.starts_with(k))
    {
        return body.contains('(') || body.ends_with(':') || body.ends_with('{');
    }
    if body.starts_with("import ") {
        return !body.contains(" from ") && !body.contains(" to ") && !body.contains(',');
    }
    if body.starts_with("from ") {
        return body.contains(" import ");
    }
    if body.starts_with("if ") || body.starts_with("while ") || body.starts_with("for ") {
        return body.ends_with(':')
            || body.ends_with('{')
            || body.contains("==")
            || body.contains("!=")
            || body.contains("<=")
            || body.contains(">=");
    }
    if body.starts_with("let ") || body.starts_with("var ") || body.starts_with("const ") {
        return body.contains('=') || body.ends_with(';');
    }
    if body.starts_with("return ") {
        return body.ends_with(';')
            || body.contains('(')
            || body.contains("None")
            || body.contains("True")
            || body.contains("False");
    }
    false
}

/// Returns `true` when `line` looks like a debug print in a non-output context.
fn is_debug_print_line(line: &str) -> bool {
    let t = line.trim_start();
    const PRINTS: &[&str] = &[
        "print(",
        "console.log(",
        "fmt.Print",
        "System.out.print",
        "puts ",
        "dbg!(",
        "println!(",
    ];
    PRINTS.iter().any(|&p| t.starts_with(p))
}

/// Returns the `TodoKind` for a `TODO`/`FIXME`/`HACK`/`XXX` marker if one
/// appears inside a comment on this line (case-insensitive word-boundary check).
fn placeholder_kind(line: &str) -> Option<TodoKind> {
    placeholder_kind_in_text(extract_comment(line)?)
}

/// Checks whether `text` contains any `TODO`/`FIXME`/`HACK`/`XXX` marker.
fn placeholder_kind_in_text(text: &str) -> Option<TodoKind> {
    let upper: String = text.chars().map(|c| c.to_ascii_uppercase()).collect();
    const MARKERS: &[(&str, TodoKind)] = &[
        ("TODO", TodoKind::Todo),
        ("FIXME", TodoKind::Fixme),
        ("HACK", TodoKind::Hack),
        ("XXX", TodoKind::Xxx),
    ];
    MARKERS
        .iter()
        .find_map(|&(tok, kind)| contains_word(&upper, tok).then_some(kind))
}

/// Word-boundary check: returns `true` when `word` appears in `text` as a
/// standalone token (surrounded by non-alphanumeric / non-underscore chars).
fn contains_word(text: &str, word: &str) -> bool {
    let (mut start, bytes) = (0usize, text.as_bytes());
    while let Some(pos) = text[start..].find(word) {
        let abs = start + pos;
        let before_ok =
            abs == 0 || (!bytes[abs - 1].is_ascii_alphanumeric() && bytes[abs - 1] != b'_');
        let after = abs + word.len();
        let after_ok =
            after >= text.len() || (!bytes[after].is_ascii_alphanumeric() && bytes[after] != b'_');
        if before_ok && after_ok {
            return true;
        }
        start = abs + 1;
    }
    false
}

/// Scans a single source file's text and appends matches to `out`.
pub fn scan_file_content(path: &Path, content: &str, skip_ctx: bool, out: &mut Vec<TodoMatch>) {
    let mut push_match = |line_num: usize, line: &str, kind: TodoKind| {
        out.push(TodoMatch {
            file: path.to_path_buf(),
            line: line_num,
            kind,
            description: kind.description().to_owned(),
            snippet: truncate_snippet(line.trim()).to_owned(),
        });
    };

    let mut in_docstring: Option<&'static str> = None;

    for (idx, line) in content.lines().enumerate() {
        let line_num = idx + 1;
        let trimmed = line.trim_start();

        if let Some(delim) = in_docstring {
            if let Some(pos) = line.find(delim) {
                in_docstring = None;
                if let Some(kind) = placeholder_kind_in_text(&line[..pos]) {
                    push_match(line_num, line, kind);
                }
            } else if let Some(kind) = placeholder_kind_in_text(line) {
                push_match(line_num, line, kind);
            }
            continue;
        }

        let doc_delim = {
            let s = trimmed
                .strip_prefix(|c: char| matches!(c, 'r' | 'R' | 'u' | 'U' | 'b' | 'B' | 'f' | 'F'))
                .unwrap_or(trimmed);
            let s = s
                .strip_prefix(|c: char| matches!(c, 'r' | 'R' | 'b' | 'B' | 'f' | 'F'))
                .unwrap_or(s);
            s.strip_prefix("\"\"\"")
                .map(|r| ("\"\"\"", r))
                .or_else(|| s.strip_prefix("'''").map(|r| ("'''", r)))
        };

        if let Some((delim, rest)) = doc_delim {
            if let Some(pos) = rest.find(delim) {
                if let Some(kind) = placeholder_kind_in_text(&rest[..pos]) {
                    push_match(line_num, line, kind);
                }
            } else {
                in_docstring = Some(delim);
                if let Some(kind) = placeholder_kind_in_text(rest) {
                    push_match(line_num, line, kind);
                }
            }
            continue;
        }

        if let Some(kind) = placeholder_kind(line) {
            push_match(line_num, line, kind);
            continue;
        }

        if !skip_ctx && is_debug_print_line(line) {
            push_match(line_num, line, TodoKind::DebugPrint);
            continue;
        }

        if is_commented_code_line(line) {
            push_match(line_num, line, TodoKind::CommentedCode);
        }
    }
}

/// Scans a set of source files in parallel and returns an aggregated `TodosResult`.
pub fn scan_files(paths: &[PathBuf]) -> TodosResult {
    let mut all_matches: Vec<TodoMatch> = paths
        .par_iter()
        .filter_map(|path| {
            let content = std::fs::read_to_string(path).ok()?;
            let mut matches = Vec::new();
            scan_file_content(path, &content, skip_context_sensitive(path), &mut matches);
            Some(matches)
        })
        .flatten()
        .collect();

    all_matches.sort_by(|a, b| a.file.cmp(&b.file).then(a.line.cmp(&b.line)));

    let mut stats = TodoStats::default();
    let mut affected = HashSet::new();
    for m in &all_matches {
        stats.increment(m.kind);
        affected.insert(&m.file);
    }
    stats.affected_files = affected.len();
    TodosResult {
        stats,
        matches: all_matches,
    }
}

/// Top-level entry point: discovers source files under `roots`, applies
/// `exclude` patterns, then delegates to [`scan_files`].
pub fn scan_todos(roots: &[PathBuf], exclude: &[String], verbose: bool) -> TodosResult {
    let files = find_python_files(roots, exclude, verbose);
    scan_files(&files)
}
