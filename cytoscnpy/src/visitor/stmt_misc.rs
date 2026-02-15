use super::*;

impl<'a> CytoScnPyVisitor<'a> {
    pub(super) fn handle_return_stmt(&mut self, node: &ast::StmtReturn) {
        if let Some(value) = &node.value {
            if let Expr::Name(name_node) = &**value {
                if name_node.ctx.is_load() {
                    let name = name_node.id.to_string();
                    if let Some(qualified) = self.resolve_name(&name) {
                        self.add_ref(qualified);
                    }
                }
            }
            self.visit_expr(value);
        }
    }

    pub(super) fn handle_assert_stmt(&mut self, node: &ast::StmtAssert) {
        self.visit_expr(&node.test);
        if let Some(msg) = &node.msg {
            self.visit_expr(msg);
        }
    }

    pub(super) fn handle_raise_stmt(&mut self, node: &ast::StmtRaise) {
        if let Some(exc) = &node.exc {
            self.visit_expr(exc);
        }
        if let Some(cause) = &node.cause {
            self.visit_expr(cause);
        }
    }

    pub(super) fn handle_delete_stmt(&mut self, node: &ast::StmtDelete) {
        for target in &node.targets {
            self.visit_expr(target);
        }
    }

    pub(super) fn handle_global_stmt(&mut self, node: &ast::StmtGlobal) {
        for name in &node.names {
            if let Some(scope) = self.scope_stack.last_mut() {
                scope.global_declarations.insert(name.id.to_string());
            }
            self.add_ref(name.id.to_string());
            if !self.module_name.is_empty() {
                let qualified = format!("{}.{}", self.module_name, name.id);
                self.add_ref(qualified);
            }
        }
    }
}
