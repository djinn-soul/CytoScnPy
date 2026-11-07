//! Quick fixes: the analyzer's dead-code fixes and `# noqa` suppressions.

use std::collections::HashMap;

use lsp_types::{
    CodeAction, CodeActionKind, CodeActionOrCommand, Diagnostic, NumberOrString, Range, TextEdit,
    Uri, WorkspaceEdit,
};

use crate::diagnostics::{suppression_code, Entry, SOURCE};
use crate::position::LineIndex;

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
        actions.push(action(
            context,
            title.clone(),
            &entry.diagnostic,
            edit,
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
        actions.push(action(context, title, diagnostic, edit, false));
    }
    actions
}

// `WorkspaceEdit::changes` is keyed by `Uri`, whose lazily parsed parts sit
// behind interior mutability that never affects its hash.
#[allow(clippy::mutable_key_type)]
fn action(
    context: &Context<'_>,
    title: String,
    diagnostic: &Diagnostic,
    edit: TextEdit,
    preferred: bool,
) -> CodeActionOrCommand {
    let changes = HashMap::from([(context.uri.clone(), vec![edit])]);
    CodeActionOrCommand::CodeAction(CodeAction {
        title,
        kind: Some(CodeActionKind::QUICKFIX),
        diagnostics: Some(vec![diagnostic.clone()]),
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
