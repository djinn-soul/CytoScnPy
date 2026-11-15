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

/// Every edit of the action titled `title`.
fn edits_of(actions: &[CodeActionOrCommand], title: &str) -> Option<Vec<TextEdit>> {
    actions.iter().find_map(|action| match action {
        CodeActionOrCommand::CodeAction(action) if action.title == title => action
            .edit
            .as_ref()
            .and_then(|edit| edit.changes.as_ref())
            .and_then(|changes| changes.values().next().cloned()),
        _ => None,
    })
}

#[test]
fn offers_fix_all_actions_from_the_analyzer_plans() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("a.py");
    let text = "import os\nimport sys\n\n\ndef unused():\n    return 1\n\n\nprint(1)\n";
    std::fs::write(&file, text).unwrap();
    let analysis = crate::analysis::analyze(&file, &[]).unwrap();
    let found = entries(&analysis.report, &analysis.text);
    let uri = Uri::from_str("file:///project/a.py").unwrap();
    let context = Context {
        uri: &uri,
        text,
        fresh_entries: Some(&found),
    };

    let actions = code_actions(&context, line(0), &[]);
    let titles: Vec<_> = summarize(&actions)
        .into_iter()
        .map(|(title, _)| title)
        .collect();
    // The function is the only unused one, so it gets no rule-wide action.
    assert_eq!(
        titles[titles.len() - 2..],
        [
            "Remove all unused imports in this file",
            "Remove all dead code in this file"
        ]
    );
    // The text is ASCII, so a character is a byte.
    let offset = |at: Position| {
        let line_start: usize = text
            .split_inclusive('\n')
            .take(at.line as usize)
            .map(str::len)
            .sum();
        line_start + at.character as usize
    };
    let apply = |edits: Vec<TextEdit>| {
        let mut fixed = text.to_owned();
        let mut edits: Vec<_> = edits
            .into_iter()
            .map(|edit| {
                (
                    offset(edit.range.start),
                    offset(edit.range.end),
                    edit.new_text,
                )
            })
            .collect();
        edits.sort_by_key(|(start, _, _)| std::cmp::Reverse(*start));
        for (start, end, new_text) in edits {
            fixed.replace_range(start..end, &new_text);
        }
        fixed
    };
    let imports = edits_of(&actions, "Remove all unused imports in this file").unwrap();
    assert_eq!(
        apply(imports),
        "def unused():\n    return 1\n\n\nprint(1)\n"
    );
    let all = edits_of(&actions, "Remove all dead code in this file").unwrap();
    assert_eq!(apply(all), "print(1)\n");

    // An edited buffer no longer matches the plans.
    let edited = format!("# header\n{text}");
    let context = Context {
        text: &edited,
        fresh_entries: None,
        ..context
    };
    assert!(edits_of(
        &code_actions(&context, line(0), &[]),
        "Remove all dead code in this file"
    )
    .is_none());
}
