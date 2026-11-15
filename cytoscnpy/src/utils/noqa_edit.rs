//! Edits that make a line's `# noqa` comment suppress one more rule.
//!
//! Mirrors `createSuppressionAction` in the VS Code extension's
//! `quickFixHelpers.ts`, so every editor writes the same comment.

use super::suppression::{comments, is_rule_code};

/// Where to insert text so that line `line` (1-indexed) of `source`
/// suppresses `code`: a byte offset into `source` and the text to insert.
///
/// A `# noqa: A, B` comment gains `, code` after its last code, ahead of any
/// explanation; `# noqa: reason` gains `code, ` before the reason; any other
/// line gets `  # noqa: code` at its end. Returns `None` when the line
/// already suppresses `code` or does not exist.
#[must_use]
pub fn noqa_insertion(source: &str, line: usize, code: &str) -> Option<(usize, String)> {
    let comment = comments(source).find(|(number, _)| *number == line);
    if let Some((_, comment)) = comment {
        // `comments` yields slices of `source`, so the offset is exact.
        let comment_start = comment.as_ptr() as usize - source.as_ptr() as usize;
        if let Some(caps) = crate::constants::SUPPRESSION_RE().captures(comment) {
            if caps.get(1).is_some() {
                return None; // `# pragma: no cytoscnpy` suppresses everything.
            }
            let codes = caps.get(2)?; // Bare `# noqa` suppresses everything.
            if codes.as_str().trim().is_empty() {
                return None;
            }
            let codes_start = comment_start + codes.start();
            let mut codes_end = None;
            for (offset, token) in tokens(codes.as_str()) {
                if !is_rule_code(token) {
                    break;
                }
                if token.eq_ignore_ascii_case("CSP") || token.eq_ignore_ascii_case(code) {
                    return None;
                }
                codes_end = Some(codes_start + offset + token.len());
            }
            return Some(match codes_end {
                Some(end) => (end, format!(", {code}")),
                None => (codes_start, format!("{code}, ")),
            });
        }
    }
    line_end(source, line).map(|end| (end, format!("  # noqa: {code}")))
}

/// Tokens separated by commas or whitespace, with their byte offsets.
fn tokens(text: &str) -> impl Iterator<Item = (usize, &str)> {
    let is_separator = |c: char| c == ',' || c.is_whitespace();
    let mut rest = 0;
    std::iter::from_fn(move || {
        let start = rest + text[rest..].find(|c: char| !is_separator(c))?;
        let end = text[start..]
            .find(is_separator)
            .map_or(text.len(), |offset| start + offset);
        rest = end;
        Some((start, &text[start..end]))
    })
}

/// Byte offset of the end of line `line` (1-indexed), before any `\r\n`.
fn line_end(source: &str, line: usize) -> Option<usize> {
    let mut start = 0;
    for _ in 1..line {
        start += source[start..].find('\n')? + 1;
    }
    if line == 0 {
        return None;
    }
    let end = source[start..]
        .find('\n')
        .map_or(source.len(), |offset| start + offset);
    Some(if source[..end].ends_with('\r') {
        end - 1
    } else {
        end
    })
}

#[cfg(test)]
mod tests {
    use super::noqa_insertion;

    fn apply(source: &str, line: usize, code: &str) -> Option<String> {
        noqa_insertion(source, line, code).map(|(offset, text)| {
            let mut edited = source.to_owned();
            edited.insert_str(offset, &text);
            edited
        })
    }

    #[test]
    fn appends_a_new_comment_to_a_line_without_one() {
        let source = "import os\r\nx = 1\n";
        assert_eq!(
            apply(source, 1, "CSP-V001").as_deref(),
            Some("import os  # noqa: CSP-V001\r\nx = 1\n")
        );
        assert_eq!(
            apply(source, 2, "F841").as_deref(),
            Some("import os\r\nx = 1  # noqa: F841\n")
        );
    }

    #[test]
    fn extends_listed_codes_ahead_of_the_explanation() {
        let source = "eval(x)  # noqa: CSP-D101, E501 trusted input\n";
        assert_eq!(
            apply(source, 1, "CSP-D001").as_deref(),
            Some("eval(x)  # noqa: CSP-D101, E501, CSP-D001 trusted input\n")
        );
    }

    #[test]
    fn adds_a_code_before_a_bare_reason() {
        let source = "eval(x)  # noqa: trusted input\n";
        assert_eq!(
            apply(source, 1, "CSP-D001").as_deref(),
            Some("eval(x)  # noqa: CSP-D001, trusted input\n")
        );
    }

    #[test]
    fn leaves_lines_that_already_suppress_the_code() {
        for source in [
            "eval(x)  # noqa\n",
            "eval(x)  # noqa: csp-d001\n",
            "eval(x)  # noqa: CSP\n",
            "eval(x)  # pragma: no cytoscnpy\n",
        ] {
            assert_eq!(noqa_insertion(source, 1, "CSP-D001"), None, "{source}");
        }
    }

    #[test]
    fn ignores_hashes_inside_strings_and_missing_lines() {
        let source = "s = \"# noqa: E501\"\n";
        assert_eq!(
            apply(source, 1, "CSP-D001").as_deref(),
            Some("s = \"# noqa: E501\"  # noqa: CSP-D001\n")
        );
        assert_eq!(noqa_insertion(source, 3, "CSP-D001"), None);
        assert_eq!(noqa_insertion("x = 1", 2, "CSP-D001"), None);
    }
}
