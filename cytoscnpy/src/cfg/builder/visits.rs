use ruff_python_ast::{self as ast, Stmt};
use ruff_text_size::Ranged;

use super::CfgBuilder;
use crate::cfg::StmtKind;

impl CfgBuilder<'_> {
    #[allow(clippy::match_same_arms)]
    pub(super) fn visit_stmt(&mut self, stmt: &Stmt) {
        let line = self.line_index.line_index(stmt.range().start());

        match stmt {
            Stmt::If(if_stmt) => {
                self.collect_expr_names(&if_stmt.test, line);
                self.add_stmt(StmtKind::If, line);
                self.visit_if(if_stmt);
            }
            Stmt::For(for_stmt) => {
                self.collect_expr_names(&for_stmt.target, line);
                self.collect_expr_names(&for_stmt.iter, line);
                self.add_stmt(StmtKind::For, line);
                self.visit_for(for_stmt);
            }
            Stmt::While(while_stmt) => {
                self.collect_expr_names(&while_stmt.test, line);
                self.add_stmt(StmtKind::While, line);
                self.visit_while(while_stmt);
            }
            Stmt::Try(try_stmt) => {
                self.add_stmt(StmtKind::Try, line);
                self.visit_try(try_stmt);
            }
            Stmt::With(with_stmt) => {
                for item in &with_stmt.items {
                    self.collect_expr_names(&item.context_expr, line);
                    if let Some(optional_vars) = &item.optional_vars {
                        self.collect_expr_names(optional_vars, line);
                    }
                }
                self.add_stmt(StmtKind::With, line);
                self.build_from_body(&with_stmt.body);
            }
            Stmt::Match(match_stmt) => {
                self.collect_expr_names(&match_stmt.subject, line);
                self.add_stmt(StmtKind::Match, line);
                self.visit_match(match_stmt);
            }
            _ => {
                self.collect_stmt_names(stmt, line);
                match stmt {
                    Stmt::Return(_) => {
                        self.add_stmt(StmtKind::Return, line);
                        self.current_block = self.new_block();
                    }
                    Stmt::Raise(_) => {
                        self.add_stmt(StmtKind::Raise, line);
                        self.current_block = self.new_block();
                    }
                    Stmt::Break(_) => {
                        self.add_stmt(StmtKind::Break, line);
                        if let Some(&(_, exit_id)) = self.loop_stack.last() {
                            self.add_edge(self.current_block, exit_id);
                        }
                        self.current_block = self.new_block();
                    }
                    Stmt::Continue(_) => {
                        self.add_stmt(StmtKind::Continue, line);
                        if let Some(&(header_id, _)) = self.loop_stack.last() {
                            self.add_edge(self.current_block, header_id);
                        }
                        self.current_block = self.new_block();
                    }
                    Stmt::Expr(expr_stmt) => {
                        if matches!(expr_stmt.value.as_ref(), ast::Expr::Call(_)) {
                            self.add_stmt(StmtKind::Call, line);
                        } else {
                            self.add_stmt(StmtKind::Simple, line);
                        }
                    }
                    _ => self.add_stmt(StmtKind::Simple, line),
                }
            }
        }
    }

    fn visit_if(&mut self, if_stmt: &ast::StmtIf) {
        let before_block = self.current_block;
        let then_block = self.new_block();
        self.add_edge(before_block, then_block);

        self.current_block = then_block;
        self.build_from_body(&if_stmt.body);
        let then_exit = self.current_block;

        let mut branch_exits = vec![then_exit];
        let mut prev_block = before_block;

        for clause in &if_stmt.elif_else_clauses {
            let clause_block = self.new_block();
            self.add_edge(prev_block, clause_block);

            if let Some(test) = &clause.test {
                self.current_block = clause_block;
                let line = self.line_index.line_index(clause.range().start());
                self.collect_expr_names(test, line);
            }

            self.current_block = clause_block;
            self.build_from_body(&clause.body);
            branch_exits.push(self.current_block);

            if clause.test.is_some() {
                prev_block = clause_block;
            }
        }

        let has_else = if_stmt
            .elif_else_clauses
            .last()
            .is_some_and(|c| c.test.is_none());
        if !has_else {
            branch_exits.push(prev_block);
        }

        let merge_block = self.new_block();
        for exit in branch_exits {
            self.add_edge(exit, merge_block);
        }
        self.current_block = merge_block;
    }

    fn visit_for(&mut self, for_stmt: &ast::StmtFor) {
        let before_block = self.current_block;
        let header_block = self.new_block();
        self.add_edge(before_block, header_block);
        let exit_block = self.new_block();

        self.loop_stack.push((header_block, exit_block));
        self.loop_depth += 1;
        self.blocks[header_block].loop_depth = self.loop_depth;

        let body_block = self.new_block();
        self.blocks[body_block].loop_depth = self.loop_depth;
        self.add_edge(header_block, body_block);

        self.current_block = body_block;
        self.build_from_body(&for_stmt.body);
        self.add_edge(self.current_block, header_block);
        self.add_edge(header_block, exit_block);

        if !for_stmt.orelse.is_empty() {
            let else_block = self.new_block();
            self.add_edge(header_block, else_block);
            self.current_block = else_block;
            self.build_from_body(&for_stmt.orelse);
            self.add_edge(self.current_block, exit_block);
        }

        self.loop_depth -= 1;
        self.loop_stack.pop();
        self.current_block = exit_block;
    }

    fn visit_while(&mut self, while_stmt: &ast::StmtWhile) {
        let before_block = self.current_block;
        let header_block = self.new_block();
        self.add_edge(before_block, header_block);
        let exit_block = self.new_block();

        self.loop_stack.push((header_block, exit_block));
        self.loop_depth += 1;
        self.blocks[header_block].loop_depth = self.loop_depth;

        let body_block = self.new_block();
        self.blocks[body_block].loop_depth = self.loop_depth;
        self.add_edge(header_block, body_block);

        self.current_block = body_block;
        self.build_from_body(&while_stmt.body);
        self.add_edge(self.current_block, header_block);
        self.add_edge(header_block, exit_block);

        if !while_stmt.orelse.is_empty() {
            let else_block = self.new_block();
            self.add_edge(header_block, else_block);
            self.current_block = else_block;
            self.build_from_body(&while_stmt.orelse);
            self.add_edge(self.current_block, exit_block);
        }

        self.loop_depth -= 1;
        self.loop_stack.pop();
        self.current_block = exit_block;
    }

    fn visit_try(&mut self, try_stmt: &ast::StmtTry) {
        let before_block = self.current_block;
        let try_block = self.new_block();
        self.add_edge(before_block, try_block);
        self.current_block = try_block;
        self.build_from_body(&try_stmt.body);
        let try_exit = self.current_block;

        let mut handler_exits = vec![try_exit];
        for handler in &try_stmt.handlers {
            let handler_block = self.new_block();
            self.add_edge(before_block, handler_block);
            self.current_block = handler_block;
            match handler {
                ast::ExceptHandler::ExceptHandler(h) => {
                    if let Some(name) = &h.name {
                        self.blocks[handler_block]
                            .defs
                            .insert((name.to_string(), 0));
                    }
                    if let Some(type_expr) = &h.type_ {
                        self.collect_expr_names(type_expr, 0);
                    }
                    self.build_from_body(&h.body);
                }
            }
            handler_exits.push(self.current_block);
        }

        if !try_stmt.orelse.is_empty() {
            let else_block = self.new_block();
            self.add_edge(try_exit, else_block);
            self.current_block = else_block;
            self.build_from_body(&try_stmt.orelse);
            handler_exits.push(self.current_block);
        }

        let merge_block = self.new_block();
        for exit in handler_exits {
            self.add_edge(exit, merge_block);
        }

        if !try_stmt.finalbody.is_empty() {
            self.current_block = merge_block;
            self.build_from_body(&try_stmt.finalbody);
        }
        self.current_block = merge_block;
    }

    fn visit_match(&mut self, match_stmt: &ast::StmtMatch) {
        let before_block = self.current_block;
        let mut case_exits = Vec::new();

        for case in &match_stmt.cases {
            let case_line = self.line_index.line_index(case.range().start());
            let pattern_block = self.new_block();
            self.add_edge(before_block, pattern_block);
            self.current_block = pattern_block;
            self.collect_pattern_names(&case.pattern, case_line);

            let mut branch_start = pattern_block;
            if let Some(guard) = &case.guard {
                let guard_block = self.new_block();
                self.add_edge(pattern_block, guard_block);
                self.current_block = guard_block;
                self.collect_expr_names(guard, case_line);
                branch_start = guard_block;
            }

            let body_block = self.new_block();
            self.add_edge(branch_start, body_block);
            self.current_block = body_block;
            self.build_from_body(&case.body);
            case_exits.push(self.current_block);
        }

        let merge_block = self.new_block();
        for exit in case_exits {
            self.add_edge(exit, merge_block);
        }
        self.current_block = merge_block;
    }
}
