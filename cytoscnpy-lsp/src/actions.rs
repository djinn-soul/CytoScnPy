//! Quick fixes: the analyzer's dead-code fixes and `# noqa` suppressions.

use std::collections::HashMap;

use lsp_types::{
    CodeAction, CodeActionKind, CodeActionOrCommand, Diagnostic, NumberOrString, Range, TextEdit,
    Uri, WorkspaceEdit,
};

use crate::diagnostics::{suppression_code, Entry, SOURCE};
use crate::position::LineIndex;
use crate::report::{Fix, FixEdit};

/// What the server knows about the document the actions are for.
pub(crate) struct Context<'a> {
    pub uri: &'a Uri,
    /// The document's current text.
    pub text: &'a str,
    /// The diagnostics from the last analysis, when `text` is still the
    /// text that analysis read; their fixes would be stale otherwise.
    pub fresh_entries: Option<&'a [Entry]>,
}

/// The actions for `range`: a fix for each fixable finding the range
/// touches or the client sent, and a suppression for each CytoScnPy
/// diagnostic the client sent.
pub(crate) fn code_actions(
    context: &Context<'_>,
    range: Range,
    client_diagnostics: &[Diagnostic],
) -> Vec<CodeActionOrCommand> {
    let index = LineIndex::new(context.text);
    let mut actions = Vec::new();
    for entry in context.fresh_entries.unwrap_or_default() {
        let Some((title, fix)) = &entry.fix else {
            continue;
        };
        let sent = client_diagnostics
            .iter()
            .any(|sent| sent.range == entry.diagnostic.range && sent.code == entry.diagnostic.code);
        if !sent && !overlaps(entry.diagnostic.range, range) {
            continue;
        }
        let edit = TextEdit::new(
            index.range(fix.start_byte, fix.end_byte),
            fix.replacement.clone(),
        );
        let diagnostics = vec![entry.diagnostic.clone()];
        actions.push(action(
            context,
            title.clone(),
            diagnostics,
            vec![edit],
            true,
        ));
    }
    for diagnostic in client_diagnostics {
        if diagnostic.source.as_deref() != Some(SOURCE) {
            continue;
        }
        let Some(NumberOrString::String(code)) = &diagnostic.code else {
            continue;
        };
        let Some(noqa_code) = suppression_code(code) else {
            continue;
        };
        let line = diagnostic.range.start.line as usize + 1;
        let Some((offset, text)) = cytoscnpy::utils::noqa_insertion(context.text, line, &noqa_code)
        else {
            continue;
        };
        let at = index.position(offset);
        let edit = TextEdit::new(Range::new(at, at), text);
        let title = format!("Suppress with # noqa: {noqa_code}");
        let diagnostics = vec![diagnostic.clone()];
        actions.push(action(context, title, diagnostics, vec![edit], false));
    }
    actions.extend(fix_all_actions(context, &index));
    actions
}

/// Like the VS Code extension, "Remove all unused …" for each rule with
/// several fixable findings and "Remove all dead code" for the whole file,
/// from the analyzer's combined plans. Fixing each finding on its own at
/// once could leave an empty suite or class body, so only those plans are
/// used.
fn fix_all_actions(context: &Context<'_>, index: &LineIndex<'_>) -> Vec<CodeActionOrCommand> {
    let fixable: Vec<(&Entry, &Fix)> = context
        .fresh_entries
        .unwrap_or_default()
        .iter()
        .filter_map(|entry| entry.fix.as_ref().map(|(_, fix)| (entry, fix)))
        .collect();
    let mut rules: Vec<&str> = Vec::new();
    for (entry, _) in &fixable {
        if let Some(NumberOrString::String(code)) = &entry.diagnostic.code {
            if !rules.contains(&code.as_str()) {
                rules.push(code);
            }
        }
    }
    let mut actions = Vec::new();
    for rule in rules {
        let Some(plural) = plural(rule) else {
            continue;
        };
        let group: Vec<_> = fixable
            .iter()
            .filter(|(entry, _)| {
                entry.diagnostic.code == Some(NumberOrString::String(rule.to_owned()))
            })
            .copied()
            .collect();
        let title = format!("Remove all unused {plural} in this file");
        actions.extend(fix_all(context, index, title, &group, |fix| {
            &fix.rule_fix_all
        }));
    }
    let title = "Remove all dead code in this file".to_owned();
    actions.extend(fix_all(context, index, title, &fixable, |fix| {
        &fix.file_fix_all
    }));
    actions
}

fn fix_all(
    context: &Context<'_>,
    index: &LineIndex<'_>,
    title: String,
    group: &[(&Entry, &Fix)],
    plan: impl Fn(&Fix) -> &Vec<FixEdit>,
) -> Option<CodeActionOrCommand> {
    if group.len() < 2 {
        return None;
    }
    let plan = group
        .iter()
        .map(|(_, fix)| plan(fix))
        .find(|edits| !edits.is_empty())?;
    let edits = plan
        .iter()
        .map(|edit| {
            context.text.get(edit.start_byte..edit.end_byte)?;
            Some(TextEdit::new(
                index.range(edit.start_byte, edit.end_byte),
                edit.replacement.clone(),
            ))
        })
        .collect::<Option<Vec<_>>>()?;
    let diagnostics = group
        .iter()
        .map(|(entry, _)| entry.diagnostic.clone())
        .collect();
    Some(action(context, title, diagnostics, edits, false))
}

/// The plural VS Code's fix-all titles use for an unused-code rule.
fn plural(rule: &str) -> Option<&'static str> {
    Some(match rule {
        "unused-function" => "functions",
        "unused-method" => "methods",
        "unused-class" => "classes",
        "unused-import" => "imports",
        "unused-variable" => "variables",
        "unused-parameter" => "parameters",
        _ => return None,
    })
}

// `WorkspaceEdit::changes` is keyed by `Uri`, whose lazily parsed parts sit
// behind interior mutability that never affects its hash.
#[allow(clippy::mutable_key_type)]
fn action(
    context: &Context<'_>,
    title: String,
    diagnostics: Vec<Diagnostic>,
    edits: Vec<TextEdit>,
    preferred: bool,
) -> CodeActionOrCommand {
    let changes = HashMap::from([(context.uri.clone(), edits)]);
    CodeActionOrCommand::CodeAction(CodeAction {
        title,
        kind: Some(CodeActionKind::QUICKFIX),
        diagnostics: Some(diagnostics),
        edit: Some(WorkspaceEdit {
            changes: Some(changes),
            ..WorkspaceEdit::default()
        }),
        is_preferred: preferred.then_some(true),
        ..CodeAction::default()
    })
}

/// Whether two ranges share a position, counting touching ends, so a
/// cursor at either end of a diagnostic gets its actions.
fn overlaps(a: Range, b: Range) -> bool {
    a.start <= b.end && b.start <= a.end
}

#[cfg(test)]
#[path = "actions_tests.rs"]
mod tests;
