#![allow(clippy::unwrap_used, clippy::mutable_key_type)]

use std::str::FromStr;

use lsp_types::{CodeActionOrCommand, Position, Range, TextEdit, Uri};

use super::{code_actions, Context};
use crate::diagnostics::entries;
use crate::report::Report;

const TEXT: &str = "import os\neval(x)  # noqa: E501\n";

fn report() -> Report {
    serde_json::from_str(
        r#"{"unused_imports": [{"line": 1, "simple_name": "os", "start_byte": 7,
            "def_type": "import", "fix": {"start_byte": 0, "end_byte": 10, "replacement": ""}}],
            "danger": [{"rule_id": "CSP-D001", "message": "Avoid using eval",
            "severity": "HIGH", "line": 2}]}"#,
    )
    .unwrap()
}

/// Each action's title and its single edit.
fn summarize(actions: &[CodeActionOrCommand]) -> Vec<(String, TextEdit)> {
    actions
        .iter()
        .map(|action| {
            let CodeActionOrCommand::CodeAction(action) = action else {
                return (
                    String::new(),
                    TextEdit::new(Range::default(), String::new()),
                );
            };
            let changes = action
                .edit
                .as_ref()
                .and_then(|e| e.changes.as_ref())
                .unwrap();
            let edit = changes.values().next().unwrap()[0].clone();
            (action.title.clone(), edit)
        })
        .collect()
}

fn line(number: u32) -> Range {
    Range::new(Position::new(number, 0), Position::new(number, 0))
}

#[test]
fn fixes_and_suppresses_findings_under_the_cursor() {
    let uri = Uri::from_str("file:///project/a.py").unwrap();
    let found = entries(&report(), TEXT);
    let diagnostics: Vec<_> = found.iter().map(|entry| entry.diagnostic.clone()).collect();
    let context = Context {
        uri: &uri,
        text: TEXT,
        fresh_entries: Some(&found),
    };

    let actions = summarize(&code_actions(&context, line(0), &diagnostics[..1]));
    assert_eq!(
        actions,
        [
            (
                "Remove unused import 'os'".to_owned(),
                TextEdit::new(
                    Range::new(Position::new(0, 0), Position::new(1, 0)),
                    String::new()
                ),
            ),
            (
                "Suppress with # noqa: unused-import".to_owned(),
                TextEdit::new(
                    Range::new(Position::new(0, 9), Position::new(0, 9)),
                    "  # noqa: unused-import".to_owned(),
                ),
            ),
        ]
    );

    // The existing `# noqa` comment gains the code after its last one.
    let actions = summarize(&code_actions(&context, line(1), &diagnostics[1..]));
    assert_eq!(
        actions,
        [(
            "Suppress with # noqa: CSP-D001".to_owned(),
            TextEdit::new(
                Range::new(Position::new(1, 21), Position::new(1, 21)),
                ", CSP-D001".to_owned(),
            ),
        )]
    );
}

#[test]
fn edited_documents_keep_suppressions_but_drop_fixes() {
    let uri = Uri::from_str("file:///project/a.py").unwrap();
    let found = entries(&report(), TEXT);
    let mut diagnostics: Vec<_> = found.iter().map(|entry| entry.diagnostic.clone()).collect();
    // Another tool's diagnostic gets no CytoScnPy action.
    diagnostics[1].source = Some("ruff".to_owned());
    let edited = format!("# header\n{TEXT}");
    let context = Context {
        uri: &uri,
        text: &edited,
        fresh_entries: None,
    };
    let mut moved = diagnostics.clone();
    moved[0].range = line(1);

    let actions = summarize(&code_actions(&context, line(1), &moved));
    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0].0, "Suppress with # noqa: unused-import");
    assert_eq!(actions[0].1.range.start, Position::new(1, 9));
}
