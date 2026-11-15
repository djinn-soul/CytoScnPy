//! Report findings to LSP diagnostics.
//!
//! Severities, tags and messages follow the VS Code extension. Rule findings
//! cover their line from its first non-blank character, as their columns are
//! not reliable; unused definitions cover just their name.

use lsp_types::{Diagnostic, DiagnosticSeverity, DiagnosticTag, NumberOrString};

use crate::position::LineIndex;
use crate::report::{Fix, Report, RuleFinding, Unused};

pub(crate) const SOURCE: &str = "cytoscnpy";

/// A diagnostic with what its code actions need.
#[derive(Debug, Clone)]
pub(crate) struct Entry {
    pub diagnostic: Diagnostic,
    /// The analyzer's fix and the title of its action.
    pub fix: Option<(String, Fix)>,
}

/// The diagnostics for a report on `text`.
pub(crate) fn entries(report: &Report, text: &str) -> Vec<Entry> {
    let index = LineIndex::new(text);
    let mut entries: Vec<Entry> = report
        .unused()
        .map(|unused| unused_entry(unused, &index))
        .collect();
    // Taint findings repeat a danger finding with the data flow added.
    let tainted: Vec<_> = report
        .taint_findings
        .iter()
        .map(|taint| (taint.rule_id.as_str(), taint.sink_line))
        .collect();
    let danger = report
        .danger
        .iter()
        .filter(|finding| !tainted.contains(&(finding.rule_id.as_str(), finding.line)));
    for finding in report.secrets.iter().chain(danger).chain(&report.quality) {
        entries.push(rule_entry(finding, &index));
    }
    for clone in &report.clones {
        entries.push(rule_entry(clone, &index));
    }
    for taint in &report.taint_findings {
        let finding = RuleFinding {
            rule_id: taint.rule_id.clone(),
            message: taint.message(),
            severity: taint.severity.clone(),
            line: taint.sink_line,
            ..RuleFinding::default()
        };
        entries.push(rule_entry(&finding, &index));
    }
    for failure in &report.parse_errors {
        let mut diagnostic = diagnostic(
            index.line_range(failure.line()),
            format!("Parse error: {}", failure.error),
            DiagnosticSeverity::ERROR,
            "parse-error",
        );
        if diagnostic.range == lsp_types::Range::default() {
            diagnostic.range = index.line_range(1);
        }
        entries.push(Entry {
            diagnostic,
            fix: None,
        });
    }
    entries
}

fn unused_entry(unused: &Unused, index: &LineIndex<'_>) -> Entry {
    let rule_id = unused.rule_id();
    let range = match unused.start_byte {
        Some(start)
            if !unused.simple_name.is_empty() && index.has_text_at(start, &unused.simple_name) =>
        {
            index.range(start, start + unused.simple_name.len())
        }
        _ => index.line_range(unused.line),
    };
    let message = unused
        .message
        .clone()
        .unwrap_or_else(|| format!("'{}' is defined but never used", unused.simple_name));
    let mut diagnostic = diagnostic(range, message, DiagnosticSeverity::WARNING, &rule_id);
    diagnostic.tags = Some(vec![DiagnosticTag::UNNECESSARY]);
    let fix = unused
        .fix
        .clone()
        .filter(|fix| fix_fits(fix, index.text()))
        .map(|fix| {
            let label = &unused.def_type;
            let name = &unused.simple_name;
            let renames = unused.def_type == "variable"
                && !fix.replacement.is_empty()
                && fix
                    .replacement
                    .chars()
                    .all(|c| c.is_alphanumeric() || c == '_');
            let title = if renames {
                format!("Rename unused {label} '{name}' to '{}'", fix.replacement)
            } else {
                format!("Remove unused {label} '{name}'")
            };
            (title, fix)
        });
    Entry { diagnostic, fix }
}

fn rule_entry(finding: &RuleFinding, index: &LineIndex<'_>) -> Entry {
    let severity = match finding.severity.to_ascii_uppercase().as_str() {
        "CRITICAL" | "HIGH" | "ERROR" => DiagnosticSeverity::ERROR,
        _ => DiagnosticSeverity::WARNING,
    };
    let diagnostic = diagnostic(
        index.line_range(finding.line),
        finding.message.clone(),
        severity,
        &finding.rule_id,
    );
    Entry {
        diagnostic,
        fix: None,
    }
}

/// The code a `# noqa` comment names to suppress a diagnostic with code
/// `code`: the code itself when the CLI honors it, otherwise `CSP`, which
/// suppresses every finding on the line. Parse errors cannot be suppressed.
pub(crate) fn suppression_code(code: &str) -> Option<String> {
    if code == "parse-error" {
        return None;
    }
    let bytes = code.as_bytes();
    let is_rule = bytes.len() == 8
        && code.is_char_boundary(4)
        && code[..4].eq_ignore_ascii_case("CSP-")
        && bytes[4].is_ascii_alphabetic()
        && bytes[5..].iter().all(u8::is_ascii_digit);
    let is_unused = code.strip_prefix("unused-").is_some_and(|kind| {
        [
            "function",
            "method",
            "class",
            "import",
            "variable",
            "parameter",
        ]
        .contains(&kind)
    });
    Some(if is_rule || is_unused {
        code.to_owned()
    } else {
        "CSP".to_owned()
    })
}

fn fix_fits(fix: &Fix, text: &str) -> bool {
    fix.start_byte <= fix.end_byte
        && text.is_char_boundary(fix.start_byte)
        && text.is_char_boundary(fix.end_byte)
}

fn diagnostic(
    range: lsp_types::Range,
    message: String,
    severity: DiagnosticSeverity,
    code: &str,
) -> Diagnostic {
    Diagnostic {
        range,
        severity: Some(severity),
        code: Some(NumberOrString::String(code.to_owned())),
        source: Some(SOURCE.to_owned()),
        message,
        ..Diagnostic::default()
    }
}

#[cfg(test)]
#[path = "diagnostics_tests.rs"]
mod tests;
