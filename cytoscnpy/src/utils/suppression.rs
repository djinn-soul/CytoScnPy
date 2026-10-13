//! Inline suppression comments (`# noqa`, `# ignore`, `# pragma: no cytoscnpy`).
//!
//! The VS Code extension mirrors this parser in
//! `editors/vscode/cytoscnpy/src/suppression.ts`; keep the two in sync.

use rustc_hash::{FxHashMap, FxHashSet};

/// Suppression specification.
#[derive(Debug, Clone, PartialEq)]
pub enum Suppression {
    /// Suppress all findings.
    All,
    /// Suppress findings for specific rule IDs.
    Specific(FxHashSet<String>),
}

impl Suppression {
    /// Returns whether this suppression covers the requested rule ID.
    #[must_use]
    pub fn covers(&self, rule_id: &str) -> bool {
        match self {
            Self::All => true,
            // Codes are stored uppercase; avoid allocating when the ID
            // already is.
            Self::Specific(rules) => {
                if rule_id.bytes().any(|b| b.is_ascii_lowercase()) {
                    rules.contains(&rule_id.to_ascii_uppercase())
                } else {
                    rules.contains(rule_id)
                }
            }
        }
    }
}

/// Yields `(line_number, comment_text)` for each line with a comment.
///
/// String literals are tracked across lines, so a `#` inside a string,
/// including a triple-quoted string spanning lines, is not a comment.
fn comments(source: &str) -> Comments<'_> {
    Comments {
        source,
        line: 1,
        open: None,
        i: 0,
    }
}

struct Comments<'a> {
    source: &'a str,
    line: usize,
    // Open string literal: its quote byte and whether it is triple-quoted.
    open: Option<(u8, bool)>,
    i: usize,
}

impl<'a> Iterator for Comments<'a> {
    type Item = (usize, &'a str);

    fn next(&mut self) -> Option<Self::Item> {
        let source = self.source;
        let bytes = source.as_bytes();
        while self.i < bytes.len() {
            let i = self.i;
            let byte = bytes[i];
            if byte == b'\n' {
                self.line += 1;
                // A single-quoted string cannot span lines without a backslash.
                if self.open.is_some_and(|(_, triple)| !triple) {
                    self.open = None;
                }
                self.i += 1;
                continue;
            }
            match self.open {
                Some((quote, triple)) => {
                    if byte == b'\\' {
                        // Skip the escaped character; a line continuation still
                        // starts a new line.
                        let skip = if bytes[i + 1..].starts_with(b"\r\n") {
                            3
                        } else {
                            2
                        };
                        if bytes.get(i + skip - 1) == Some(&b'\n') {
                            self.line += 1;
                        }
                        self.i += skip;
                        continue;
                    }
                    if byte == quote {
                        if !triple {
                            self.open = None;
                        } else if bytes[i..].starts_with(&[quote; 3]) {
                            self.open = None;
                            self.i += 3;
                            continue;
                        }
                    }
                    self.i += 1;
                }
                None => match byte {
                    b'#' => {
                        let end = source[i..]
                            .find('\n')
                            .map_or(source.len(), |offset| i + offset);
                        self.i = end;
                        return Some((self.line, source[i..end].trim_end_matches('\r')));
                    }
                    b'"' | b'\'' => {
                        let triple = bytes[i..].starts_with(&[byte; 3]);
                        self.open = Some((byte, triple));
                        self.i += if triple { 3 } else { 1 };
                    }
                    _ => self.i += 1,
                },
            }
        }
        None
    }
}

/// A token that can name a rule (`CSP`, `CSP-D101`, `E501`, `unused-import`).
/// The first token that cannot is where the explanation begins.
fn is_rule_code(token: &str) -> bool {
    token.starts_with(|c: char| c.is_ascii_alphabetic())
        && token
            .split('-')
            .all(|part| !part.is_empty() && part.chars().all(|c| c.is_ascii_alphanumeric()))
        && (token.eq_ignore_ascii_case("CSP")
            || token.contains('-')
            || token.chars().any(|c| c.is_ascii_digit()))
}

/// Parses the suppression directive in a comment, if any.
fn comment_suppression(comment: &str) -> Option<Suppression> {
    let caps = crate::constants::SUPPRESSION_RE().captures(comment)?;
    if caps.get(1).is_some() {
        return Some(Suppression::All);
    }
    let Some(codes) = caps.get(2).map(|m| m.as_str()) else {
        // Bare `# noqa` or `# ignore`.
        return Some(Suppression::All);
    };
    if codes.trim().is_empty() {
        return Some(Suppression::All);
    }
    // `# noqa: CSP-D101, E501 reason` lists codes, then explains them.
    let mut rules = FxHashSet::default();
    for code in codes
        .split(|c: char| c == ',' || c.is_whitespace())
        .filter(|token| !token.is_empty())
        .take_while(|token| is_rule_code(token))
    {
        let code = code.to_uppercase();
        if code == "CSP" {
            return Some(Suppression::All);
        }
        rules.insert(code);
    }
    (!rules.is_empty()).then_some(Suppression::Specific(rules))
}

/// Detects the suppression specification for a single line.
///
/// Supports multiple formats:
/// - `# pragma: no cytoscnpy` - Legacy format (All)
/// - `# noqa` or `# ignore` - Bare ignore (All)
/// - `# noqa: CSP-D101, CSP-Q202` - Specific codes
///
/// The line is read on its own, so a line inside a multi-line string is
/// treated as code. Prefer [`get_ignored_lines`] when the whole source is
/// available.
#[must_use]
pub fn get_line_suppression(line: &str) -> Option<Suppression> {
    comments(line).find_map(|(_, comment)| comment_suppression(comment))
}

/// Detects lines with suppression comments in a source file.
///
/// Returns a map of line numbers (1-indexed) to suppression specs.
#[must_use]
pub fn get_ignored_lines(source: &str) -> FxHashMap<usize, Suppression> {
    comments(source)
        .filter_map(|(line, comment)| comment_suppression(comment).map(|s| (line, s)))
        .collect()
}

/// Checks if a specific line and rule are suppressed.
///
/// Returns true if the finding should be ignored.
#[must_use]
pub fn is_line_suppressed(
    ignored_lines: &std::collections::HashMap<usize, Suppression, impl std::hash::BuildHasher>,
    line: usize,
    rule_id: &str,
) -> bool {
    ignored_lines
        .get(&line)
        .is_some_and(|suppression| suppression.covers(rule_id))
}

/// Returns whether an inline comment suppresses the requested rule ID.
#[must_use]
pub fn is_rule_suppressed_on_line(line: &str, rule_id: &str) -> bool {
    get_line_suppression(line).is_some_and(|suppression| suppression.covers(rule_id))
}
