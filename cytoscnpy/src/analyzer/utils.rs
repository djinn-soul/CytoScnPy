//! Utility functions for the analyzer.

use crate::constants::MAX_RECURSION_DEPTH;
use crate::utils::LineIndex;
use ruff_python_ast::{Expr, Stmt};

/// Collects line numbers that belong to docstrings by traversing the AST.
pub(crate) fn collect_docstring_lines(
    body: &[Stmt],
    line_index: &LineIndex,
    docstrings: &mut rustc_hash::FxHashSet<usize>,
    depth: usize,
) {
    if depth > MAX_RECURSION_DEPTH {
        return;
    }

    if let Some(Stmt::Expr(expr_stmt)) = body.first() {
        if let Expr::StringLiteral(string_lit) = &*expr_stmt.value {
            let start_line = line_index.line_index(string_lit.range.start());
            let end_line = line_index.line_index(string_lit.range.end());
            for i in start_line..=end_line {
                docstrings.insert(i);
            }
        }
    }

    for stmt in body {
        match stmt {
            Stmt::FunctionDef(f) => {
                collect_docstring_lines(&f.body, line_index, docstrings, depth + 1);
            }
            Stmt::ClassDef(c) => {
                collect_docstring_lines(&c.body, line_index, docstrings, depth + 1);
            }
            _ => {}
        }
    }
}

/// Converts byte range references in error messages to line numbers.
///
/// Ruff parser errors include "at byte range X..Y" which is not user-friendly.
/// This function replaces them with "at line N" for better readability.
pub(crate) fn convert_byte_range_to_line(error_msg: &str, source: &str) -> String {
    use regex::Regex;

    // Match "at byte range X..Y" or "byte range X..Y"
    let Ok(re) = Regex::new(r"(?:at )?byte range (\d+)\.\.(\d+)") else {
        return error_msg.to_owned();
    };

    re.replace_all(error_msg, |caps: &regex::Captures| {
        if let Ok(start_byte) = caps[1].parse::<usize>() {
            // Count newlines up to start_byte to find line number
            let line = source[..start_byte.min(source.len())].matches('\n').count() + 1;
            format!("at line {line}")
        } else {
            caps[0].to_string()
        }
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::LineIndex;
    use ruff_python_parser::parse_module;

    #[test]
    fn test_stack_overflow_protection() {
        // Generate a deeply nested Python file
        let depth = 500; // Well above MAX_RECURSION_DEPTH
        let mut code = String::new();
        for i in 1..=depth {
            let indent = " ".repeat((i - 1) * 2);
            code.push_str(&format!("{indent}def f{i}():\n"));
            code.push_str(&format!("{indent}  \"\"\"Docstring {i}\"\"\"\n"));
        }
        code.push_str(&" ".repeat(depth * 2));
        code.push_str("pass\n");

        let parsed = parse_module(&code).expect("Failed to parse deeply nested code");
        let line_index = LineIndex::new(&code);
        let mut docstrings = rustc_hash::FxHashSet::default();

        // Should not crash due to MAX_RECURSION_DEPTH
        collect_docstring_lines(&parsed.into_syntax().body, &line_index, &mut docstrings, 0);

        // Should have collected some docstrings (up to limit)
        assert!(!docstrings.is_empty());
    }
}
