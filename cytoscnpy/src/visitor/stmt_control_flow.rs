use super::*;

impl<'a> CytoScnPyVisitor<'a> {
    pub(super) fn handle_if_stmt(&mut self, node: &ast::StmtIf) {
        let mut is_type_checking_guard = false;
        if let Expr::Name(name) = &*node.test {
            if name.id.as_str() == "TYPE_CHECKING" {
                is_type_checking_guard = true;
            } else if let Some(original) = self.alias_map.get(name.id.as_str()) {
                if original.ends_with("TYPE_CHECKING") {
                    is_type_checking_guard = true;
                }
            }
        } else if let Expr::Attribute(attr) = &*node.test {
            if attr.attr.as_str() == "TYPE_CHECKING" {
                if let Expr::Name(base) = &*attr.value {
                    if base.id.as_str() == "typing" || base.id.as_str() == "typing_extensions" {
                        is_type_checking_guard = true;
                    } else if let Some(original) = self.alias_map.get(base.id.as_str()) {
                        if original == "typing" || original == "typing_extensions" {
                            is_type_checking_guard = true;
                        }
                    }
                }
            }
        }

        self.visit_expr(&node.test);
        let prev_in_type_checking = self.in_type_checking_block;
        if is_type_checking_guard {
            self.in_type_checking_block = true;
        }

        for stmt in &node.body {
            self.visit_stmt(stmt);
        }
        self.in_type_checking_block = prev_in_type_checking;

        for clause in &node.elif_else_clauses {
            if let Some(test) = &clause.test {
                self.visit_expr(test);
            }
            for stmt in &clause.body {
                self.visit_stmt(stmt);
            }
        }
    }

    pub(super) fn handle_for_stmt(&mut self, node: &ast::StmtFor) {
        self.visit_expr(&node.iter);
        self.visit_definition_target(&node.target);
        for stmt in &node.body {
            self.visit_stmt(stmt);
        }
        for stmt in &node.orelse {
            self.visit_stmt(stmt);
        }
    }

    pub(super) fn handle_while_stmt(&mut self, node: &ast::StmtWhile) {
        self.visit_expr(&node.test);
        for stmt in &node.body {
            self.visit_stmt(stmt);
        }
        for stmt in &node.orelse {
            self.visit_stmt(stmt);
        }
    }

    pub(super) fn handle_with_stmt(&mut self, node: &ast::StmtWith) {
        for item in &node.items {
            self.visit_expr(&item.context_expr);
        }
        for stmt in &node.body {
            self.visit_stmt(stmt);
        }
    }

    pub(super) fn handle_try_stmt(&mut self, node: &ast::StmtTry) {
        let mut catches_import_error = false;
        for handler in &node.handlers {
            let ruff_python_ast::ExceptHandler::ExceptHandler(h) = handler;
            if let Some(type_) = &h.type_ {
                if let Expr::Name(name) = &**type_ {
                    if name.id.as_str() == "ImportError"
                        || name.id.as_str() == "ModuleNotFoundError"
                    {
                        catches_import_error = true;
                    }
                } else if let Expr::Tuple(tuple) = &**type_ {
                    for elt in &tuple.elts {
                        if let Expr::Name(name) = elt {
                            if name.id.as_str() == "ImportError"
                                || name.id.as_str() == "ModuleNotFoundError"
                            {
                                catches_import_error = true;
                            }
                        }
                    }
                }
            }
        }

        let prev_in_import_error = self.in_import_error_block;
        if catches_import_error {
            self.in_import_error_block = true;
        }

        for stmt in &node.body {
            self.visit_stmt(stmt);
        }

        self.in_import_error_block = prev_in_import_error;
        for ast::ExceptHandler::ExceptHandler(handler_node) in &node.handlers {
            if let Some(exc) = &handler_node.type_ {
                self.visit_expr(exc);
            }
            for stmt in &handler_node.body {
                self.visit_stmt(stmt);
            }
        }
        for stmt in &node.orelse {
            self.visit_stmt(stmt);
        }
        for stmt in &node.finalbody {
            self.visit_stmt(stmt);
        }
    }

    pub(super) fn handle_match_stmt(&mut self, node: &ast::StmtMatch) {
        self.visit_expr(&node.subject);
        for case in &node.cases {
            self.visit_match_pattern(&case.pattern);
            if let Some(guard) = &case.guard {
                self.visit_expr(guard);
            }
            for stmt in &case.body {
                self.visit_stmt(stmt);
            }
        }
    }
}
