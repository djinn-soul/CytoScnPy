//! Preserve required suites and separators when deleting Python statements.

use super::apply_plan::PlannedEdit;
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{ModModule, Stmt};
use ruff_text_size::Ranged;

pub(super) fn preserve_syntax(module: &ModModule, content: &str, planned: &mut Vec<PlannedEdit>) {
    let mut suites = SuitePlanner { planned };
    // The module itself may be empty; compound-statement bodies may not.
    for stmt in &module.body {
        suites.visit_stmt(stmt);
    }
    let mut separators = SeparatorPlanner { planned, content };
    for stmt in &module.body {
        separators.visit_stmt(stmt);
    }
    // Adjacent deleted statements can share a semicolon after expansion.
    let mut merged: Vec<PlannedEdit> = Vec::new();
    for edit in std::mem::take(planned) {
        if let Some(previous) = merged.last_mut() {
            if edit.start_byte < previous.end_byte
                && previous.replacement.is_none()
                && edit.replacement.is_none()
            {
                previous.end_byte = previous.end_byte.max(edit.end_byte);
                previous.removed_names.extend(edit.removed_names);
                continue;
            }
        }
        merged.push(edit);
    }
    *planned = merged;
}

struct SuitePlanner<'a> {
    planned: &'a mut [PlannedEdit],
}

impl SuitePlanner<'_> {
    fn deletion_for(&self, stmt: &Stmt) -> Option<usize> {
        self.planned.iter().position(|edit| {
            edit.replacement.is_none()
                && edit.start_byte <= stmt.range().start().to_usize()
                && edit.end_byte >= stmt.range().end().to_usize()
        })
    }
}

impl<'a> Visitor<'a> for SuitePlanner<'_> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        if self.deletion_for(stmt).is_none() {
            visitor::walk_stmt(self, stmt);
        }
    }

    fn visit_body(&mut self, body: &'a [Stmt]) {
        if let Some(first) = body.first() {
            if body.iter().all(|stmt| self.deletion_for(stmt).is_some()) {
                if let Some(index) = self.deletion_for(first) {
                    self.planned[index].replacement = Some("pass".to_owned());
                }
            }
        }
        visitor::walk_body(self, body);
    }
}

struct SeparatorPlanner<'a> {
    planned: &'a mut [PlannedEdit],
    content: &'a str,
}

impl<'a> Visitor<'a> for SeparatorPlanner<'_> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        // Only whole-statement deletions consume a separator. Removing an import
        // alias must leave the separator between the surviving import and its neighbor.
        if let Some(edit) = self.planned.iter_mut().find(|edit| {
            edit.replacement.is_none()
                && edit.start_byte <= stmt.range().start().to_usize()
                && edit.end_byte >= stmt.range().end().to_usize()
        }) {
            let bytes = self.content.as_bytes();
            let mut end = edit.end_byte;
            while end < bytes.len() && matches!(bytes[end], b' ' | b'\t') {
                end += 1;
            }
            if bytes.get(end) == Some(&b';') {
                end += 1;
                while end < bytes.len() && matches!(bytes[end], b' ' | b'\t') {
                    end += 1;
                }
                edit.end_byte = end;
            } else {
                let mut start = edit.start_byte;
                while start > 0 && matches!(bytes[start - 1], b' ' | b'\t') {
                    start -= 1;
                }
                if start > 0 && bytes[start - 1] == b';' {
                    edit.start_byte = start - 1;
                }
            }
        } else {
            visitor::walk_stmt(self, stmt);
        }
    }
}
