use crate::utils::LineIndex;
use rustpython_ast::TextSize;
use rustpython_parser::lexer;
use rustpython_parser::Tok;

#[derive(Debug, Default, Clone, PartialEq)]
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

/// Analyzes raw metrics (LOC, SLOC, etc.) from source code.
pub fn analyze_raw(code: &str) -> RawMetrics {
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

    // 2. Scan tokens to find Strings (Multi-line) and mask them to find Comments
    let lexer = lexer::Lexer::new(code.chars(), TextSize::default());
    let line_index = LineIndex::new(code);
    let mut string_ranges = Vec::new();

    for result in lexer {
        if let Ok((token, range)) = result {
            if let Tok::String { .. } = token {
                let start_row = line_index.line_index(range.start());
                let end_row = line_index.line_index(range.end());

                let start_offset = range.start().to_usize();
                let end_offset = range.end().to_usize();
                string_ranges.push((start_offset, end_offset));

                if end_row > start_row {
                    // Multi-line string
                    for r in start_row..=end_row {
                        if r <= metrics.loc
                            && line_types[r] != LineType::Blank {
                                line_types[r] = LineType::Multi;
                            }
                    }
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
            for (s, e) in &string_ranges {
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
            LineType::Multi => metrics.multi += 1,
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
