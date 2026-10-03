//! Plan import removals in contiguous groups within each statement.

use super::apply_plan::PlannedEdit;
use super::ranges::trim_comma_range;
use crate::visitor::Definition;
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{Alias, ModModule, Stmt};
use ruff_text_size::Ranged;

pub(super) fn plan_import_edits(
    items: &[(&'static str, &Definition)],
    module: &ModModule,
    content: &str,
) -> Vec<PlannedEdit> {
    let mut planner = ImportPlanner {
        items,
        content,
        planned: Vec::new(),
    };
    for stmt in &module.body {
        planner.visit_stmt(stmt);
    }
    planner.planned
}

struct ImportPlanner<'a> {
    items: &'a [(&'static str, &'a Definition)],
    content: &'a str,
    planned: Vec<PlannedEdit>,
}

impl ImportPlanner<'_> {
    fn plan_statement(&mut self, stmt: &Stmt, aliases: &[Alias]) {
        let targets: Vec<_> = aliases
            .iter()
            .map(|alias| {
                self.items.iter().find_map(|(kind, def)| {
                    (*kind == "import"
                        && def.start_byte == alias.range().start().to_usize()
                        && def.simple_name == alias.asname.as_ref().unwrap_or(&alias.name).as_str())
                    .then_some(*def)
                })
            })
            .collect();
        let mut index = 0;
        while index < aliases.len() {
            if targets[index].is_none() {
                index += 1;
                continue;
            }
            let first = index;
            while index < aliases.len() && targets[index].is_some() {
                index += 1;
            }
            let (start, end) = if first == 0 && index == aliases.len() {
                (
                    stmt.range().start().to_usize(),
                    stmt.range().end().to_usize(),
                )
            } else if index < aliases.len() {
                // Remove the group and its following commas up to the next kept alias.
                (
                    aliases[first].range().start().to_usize(),
                    aliases[index].range().start().to_usize(),
                )
            } else {
                // A final group must remove its preceding comma if there is no trailing one.
                let (start, end, _) = trim_comma_range(
                    self.content,
                    aliases[first].range().start().to_usize(),
                    aliases[index - 1].range().end().to_usize(),
                );
                (start, end)
            };
            let definitions: Vec<_> = targets[first..index].iter().flatten().copied().collect();
            let names: Vec<_> = definitions
                .iter()
                .map(|def| def.simple_name.clone())
                .collect();
            self.planned.push(PlannedEdit {
                start_byte: start,
                end_byte: end,
                replacement: None,
                name: names.join(", "),
                removed_names: names,
                item_type: "import",
                line: definitions[0].line,
            });
        }
    }
}

impl<'a> Visitor<'a> for ImportPlanner<'_> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        match stmt {
            Stmt::Import(node) => self.plan_statement(stmt, &node.names),
            Stmt::ImportFrom(node) => self.plan_statement(stmt, &node.names),
            _ => visitor::walk_stmt(self, stmt),
        }
    }
}
