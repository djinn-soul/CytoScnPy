use crate::utils::LineIndex;
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{Expr, ModModule, Stmt};
use ruff_python_parser::parse_module;
use ruff_text_size::Ranged;

use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
/// Raw metrics gathered from source code analysis.
pub struct RawMetrics {
    /// Total lines of code.
    pub loc: usize,
    /// Logical lines of code (source without empty lines).
    pub lloc: usize,
    /// Source lines of code (code lines without comments).
    pub sloc: usize,
    /// Number of comment lines.
    pub comments: usize,
    /// Number of multi-line comments.
    pub multi: usize,
    /// Number of blank lines.
    pub blank: usize,
    /// Number of single-line comments.
    pub single_comments: usize,
}

struct StringCollector {
    ranges: Vec<(usize, usize)>,
}

impl<'a> Visitor<'a> for StringCollector {
    fn visit_expr(&mut self, expr: &'a Expr) {
        self.collect_string_range(expr);
        visitor::walk_expr(self, expr);
    }
}

impl StringCollector {
    fn collect_string_range(&mut self, expr: &Expr) {
        match expr {
            Expr::StringLiteral(s) => {
                let range = s.range();
                self.ranges
                    .push((range.start().to_usize(), range.end().to_usize()));
            }
            Expr::BytesLiteral(b) => {
                let range = b.range();
                self.ranges
                    .push((range.start().to_usize(), range.end().to_usize()));
            }
            Expr::FString(f) => {
                let range = f.range();
                self.ranges
                    .push((range.start().to_usize(), range.end().to_usize()));
            }
            _ => {}
        }
    }
}

/// Collects byte ranges of all string/bytes/f-string literals in a module body.
fn collect_string_ranges(body: &[Stmt]) -> Vec<(usize, usize)> {
    let mut collector = StringCollector { ranges: Vec::new() };
    for stmt in body {
        collector.visit_stmt(stmt);
    }
    collector.ranges
}

/// Analyzes raw metrics (LOC, SLOC, etc.) from source code.
///
/// Parses `code` internally. Callers that already hold the parsed AST should
/// use `analyze_raw_with_module` to skip the reparse.
#[must_use]
pub fn analyze_raw(code: &str) -> RawMetrics {
    let string_ranges = match parse_module(code) {
        Ok(parsed) => collect_string_ranges(&parsed.into_syntax().body),
        Err(_) => Vec::new(),
    };
    analyze_raw_inner(code, &string_ranges)
}

/// Analyzes raw metrics from source code using an already-parsed module.
///
/// Avoids the full reparse that `analyze_raw` performs.
#[must_use]
pub fn analyze_raw_with_module(code: &str, module: &ModModule) -> RawMetrics {
    let string_ranges = collect_string_ranges(&module.body);
    analyze_raw_inner(code, &string_ranges)
}

fn analyze_raw_inner(code: &str, string_ranges: &[(usize, usize)]) -> RawMetrics {
    let mut metrics = RawMetrics::default();

    let mut lines: Vec<&str> = code.lines().collect();
    if code.ends_with('\n') && !code.is_empty() {
        lines.push("");
    }
    metrics.loc = lines.len();

    let mut line_types = vec![LineType::Code; metrics.loc + 1]; // 1-indexed

    // 1. Identify Blank lines
    for (i, line) in lines.iter().enumerate() {
        if line.trim().is_empty() {
            line_types[i + 1] = LineType::Blank;
            metrics.blank += 1;
        }
    }

    let line_index = LineIndex::new(code);

    for (start_offset, end_offset) in string_ranges {
        let start_row = line_index.line_index(ruff_text_size::TextSize::new(
            u32::try_from(*start_offset).unwrap_or(0),
        ));
        let end_row = line_index.line_index(ruff_text_size::TextSize::new(
            u32::try_from(*end_offset).unwrap_or(0),
        ));

        if end_row > start_row {
            // Multi-line string
            for line_type in line_types
                .iter_mut()
                .take(std::cmp::min(end_row + 1, metrics.loc + 1))
                .skip(start_row)
            {
                if *line_type != LineType::Blank {
                    *line_type = LineType::Multi;
                }
            }
        }
    }

    // 3. Find Comments
    let mut current_offset = 0;
    // We iterate over lines to check for comments.
    // We use split_inclusive to get lines with newlines to track offsets correctly.
    for (i, line_with_newline) in code.split_inclusive('\n').enumerate() {
        let line_num = i + 1;
        let line_start_offset = current_offset;
        let line_len = line_with_newline.len();
        current_offset += line_len;

        let line_content = line_with_newline.trim_end(); // Remove newline for content check

        if line_num > metrics.loc {
            break;
        }

        if line_types[line_num] == LineType::Blank {
            continue;
        }

        if let Some(idx) = line_content.find('#') {
            let hash_offset = line_start_offset + idx;

            // Check if this hash_offset is inside any string range [start, end)
            let mut is_in_string = false;
            for (s, e) in string_ranges {
                if hash_offset >= *s && hash_offset < *e {
                    is_in_string = true;
                    break;
                }
            }

            if !is_in_string {
                // It's a comment!
                let prefix = &line_content[..idx];
                if prefix.trim().is_empty() {
                    // Full line comment
                    line_types[line_num] = LineType::Comment;
                } else {
                    // Inline comment
                    metrics.single_comments += 1;
                }
            }
        }
    }

    // 4. Aggregate metrics
    metrics.multi = 0;
    metrics.comments = 0;
    metrics.sloc = 0;

    for t in line_types.iter().skip(1) {
        match t {
            LineType::Multi => {
                metrics.multi += 1;
                metrics.sloc += 1;
            }
            LineType::Comment => {
                metrics.comments += 1;
                metrics.single_comments += 1;
            }
            LineType::Code => metrics.sloc += 1,
            LineType::Blank => {}
        }
    }

    // LLOC approximation
    metrics.lloc = metrics.sloc;

    metrics
}

#[derive(Clone, PartialEq, Debug)]
enum LineType {
    Blank,
    Code,
    Multi,
    Comment,
}
