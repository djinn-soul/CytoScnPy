//! The parts of `cytoscnpy --json` output that become diagnostics.
//!
//! Every field defaults, so the server keeps working when the report gains
//! or drops fields.

use std::collections::HashMap;
use std::path::PathBuf;

use serde::Deserialize;

/// An analysis report on one file or a whole project.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub(crate) struct Report {
    pub unused_functions: Vec<Unused>,
    pub unused_methods: Vec<Unused>,
    pub unused_imports: Vec<Unused>,
    pub unused_classes: Vec<Unused>,
    pub unused_variables: Vec<Unused>,
    pub unused_parameters: Vec<Unused>,
    pub secrets: Vec<RuleFinding>,
    pub danger: Vec<RuleFinding>,
    pub quality: Vec<RuleFinding>,
    pub clones: Vec<RuleFinding>,
    pub taint_findings: Vec<TaintFinding>,
    pub parse_errors: Vec<ParseFailure>,
}

impl Report {
    /// Every unused definition, in report order.
    pub(crate) fn unused(&self) -> impl Iterator<Item = &Unused> {
        [
            &self.unused_functions,
            &self.unused_methods,
            &self.unused_imports,
            &self.unused_classes,
            &self.unused_variables,
            &self.unused_parameters,
        ]
        .into_iter()
        .flatten()
    }

    /// The findings of a project report, split by the file they are in.
    /// Findings that name no file are dropped.
    pub(crate) fn by_file(self) -> HashMap<PathBuf, Self> {
        let mut files: HashMap<PathBuf, Self> = HashMap::new();
        macro_rules! move_findings {
            ($($field:ident),*) => {$(
                for finding in self.$field {
                    if let Some(file) = finding.file.clone() {
                        files.entry(file).or_default().$field.push(finding);
                    }
                }
            )*};
        }
        move_findings!(
            unused_functions,
            unused_methods,
            unused_imports,
            unused_classes,
            unused_variables,
            unused_parameters,
            secrets,
            danger,
            quality,
            clones,
            taint_findings,
            parse_errors
        );
        files
    }
}

/// A definition that is never used.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub(crate) struct Unused {
    /// The file the finding is in.
    pub file: Option<PathBuf>,
    pub line: usize,
    pub simple_name: String,
    /// Byte offset of the name.
    pub start_byte: Option<usize>,
    /// `function`, `method`, `class`, `import`, `variable` or `parameter`.
    pub def_type: String,
    pub message: Option<String>,
    pub fix: Option<Fix>,
}

impl Unused {
    /// The rule ID, such as `unused-import`, that `# noqa` accepts.
    pub(crate) fn rule_id(&self) -> String {
        format!("unused-{}", self.def_type)
    }
}

/// An edit replacing bytes `start_byte..end_byte` of the analyzed file.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub(crate) struct Fix {
    pub start_byte: usize,
    pub end_byte: usize,
    pub replacement: String,
}

/// A secret, danger, quality or clone finding.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub(crate) struct RuleFinding {
    /// The file the finding is in.
    pub file: Option<PathBuf>,
    pub rule_id: String,
    pub message: String,
    pub severity: String,
    pub line: usize,
}

/// Tainted data reaching a dangerous sink.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub(crate) struct TaintFinding {
    /// The file the finding is in.
    pub file: Option<PathBuf>,
    pub rule_id: String,
    pub severity: String,
    pub vuln_type: String,
    pub source: String,
    pub source_line: usize,
    pub sink: String,
    pub sink_line: usize,
    pub flow_path: Vec<String>,
    pub remediation: String,
}

impl TaintFinding {
    /// Matches the VS Code extension's taint message.
    pub(crate) fn message(&self) -> String {
        let flow = std::iter::once(self.source.as_str())
            .chain(self.flow_path.iter().map(String::as_str))
            .chain(std::iter::once(self.sink.as_str()))
            .collect::<Vec<_>>()
            .join(" -> ");
        format!(
            "{}: Tainted data from {} (line {}) reaches sink {}.\n\nFlow: {flow}\n\nRemediation: {}",
            self.vuln_type, self.source, self.source_line, self.sink, self.remediation
        )
    }
}

/// A file the analyzer could not parse or read.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub(crate) struct ParseFailure {
    /// The file the finding is in.
    pub file: Option<PathBuf>,
    pub error: String,
}

impl ParseFailure {
    /// The line named by a trailing `at line N`, which parse errors carry.
    pub(crate) fn line(&self) -> usize {
        self.error
            .rsplit_once("at line ")
            .and_then(|(_, rest)| {
                let digits = rest
                    .find(|c: char| !c.is_ascii_digit())
                    .unwrap_or(rest.len());
                rest[..digits].parse().ok()
            })
            .unwrap_or(1)
    }
}

#[cfg(test)]
mod tests {
    use super::{ParseFailure, Report};

    #[test]
    fn reads_a_cli_report_and_ignores_unknown_fields() {
        let report: Report = serde_json::from_str(
            r#"{"unused_imports": [{"line": 1, "simple_name": "os", "start_byte": 7,
                "def_type": "import", "fix": {"start_byte": 0, "end_byte": 10,
                "replacement": ""}, "confidence": 100}],
                "danger": [{"rule_id": "CSP-D001", "message": "Avoid using eval",
                "severity": "HIGH", "line": 4, "col": 0}],
                "analysis_summary": {"total_files": 1}}"#,
        )
        .unwrap_or_default();
        let unused: Vec<_> = report.unused().collect();
        assert_eq!(unused.len(), 1);
        assert_eq!(unused[0].rule_id(), "unused-import");
        assert_eq!(report.danger[0].rule_id, "CSP-D001");
    }

    #[test]
    fn parse_errors_name_their_line() {
        let at = |error: &str| {
            ParseFailure {
                error: error.to_owned(),
                ..ParseFailure::default()
            }
            .line()
        };
        assert_eq!(at("Expected an expression at line 12"), 12);
        assert_eq!(at("Expected ':' at line 3, column 4"), 3);
        assert_eq!(at("Failed to read file"), 1);
    }
}
