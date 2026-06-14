use ruff_python_ast::{self as ast, Stmt};
use std::collections::HashSet;

pub(super) struct LcomVisitor {
    pub(super) used_fields: HashSet<String>,
    pub(super) called_methods: HashSet<String>,
    receiver_name: Option<String>,
}

impl LcomVisitor {
    pub(super) fn new(receiver_name: Option<String>) -> Self {
        Self {
            used_fields: HashSet::new(),
            called_methods: HashSet::new(),
            receiver_name,
        }
    }

    pub(super) fn visit_body(&mut self, body: &[Stmt]) {
        for stmt in body {
            self.visit_stmt(stmt);
        }
    }

    fn is_receiver_name(&self, name: &str) -> bool {
        self.receiver_name.as_deref() == Some(name)
    }

    fn visit_stmt(&mut self, stmt: &Stmt) {
        if self.visit_assignment_stmt(stmt) {
            return;
        }
        if self.visit_branch_stmt(stmt) {
            return;
        }
        if self.visit_context_stmt(stmt) {
            return;
        }
        let _ = self.visit_simple_stmt(stmt);
    }

    fn visit_assignment_stmt(&mut self, stmt: &Stmt) -> bool {
        match stmt {
            Stmt::Assign(node) => {
                self.visit_expr(&node.value);
                self.visit_expr_list(&node.targets);
            }
            Stmt::AugAssign(node) => {
                self.visit_expr(&node.target);
                self.visit_expr(&node.value);
            }
            _ => return false,
        }
        true
    }

    fn visit_branch_stmt(&mut self, stmt: &Stmt) -> bool {
        match stmt {
            Stmt::If(node) => {
                self.visit_expr(&node.test);
                self.visit_body(&node.body);
                for clause in &node.elif_else_clauses {
                    if let Some(test) = &clause.test {
                        self.visit_expr(test);
                    }
                    self.visit_body(&clause.body);
                }
            }
            Stmt::For(node) => {
                self.visit_expr(&node.iter);
                self.visit_body(&node.body);
                self.visit_body(&node.orelse);
            }
            Stmt::While(node) => {
                self.visit_expr(&node.test);
                self.visit_body(&node.body);
                self.visit_body(&node.orelse);
            }
            Stmt::Try(node) => self.visit_try_stmt(node),
            _ => return false,
        }
        true
    }

    fn visit_try_stmt(&mut self, node: &ast::StmtTry) {
        self.visit_body(&node.body);
        for handler in &node.handlers {
            let ast::ExceptHandler::ExceptHandler(handler) = handler;
            if let Some(type_) = &handler.type_ {
                self.visit_expr(type_);
            }
            self.visit_body(&handler.body);
        }
        self.visit_body(&node.orelse);
        self.visit_body(&node.finalbody);
    }

    fn visit_context_stmt(&mut self, stmt: &Stmt) -> bool {
        let Stmt::With(node) = stmt else {
            return false;
        };

        for item in &node.items {
            self.visit_expr(&item.context_expr);
            if let Some(optional_vars) = &item.optional_vars {
                self.visit_expr(optional_vars);
            }
        }
        self.visit_body(&node.body);
        true
    }

    fn visit_simple_stmt(&mut self, stmt: &Stmt) -> bool {
        match stmt {
            Stmt::Expr(node) => self.visit_expr(&node.value),
            Stmt::Return(node) => {
                if let Some(value) = &node.value {
                    self.visit_expr(value);
                }
            }
            _ => return false,
        }
        true
    }

    fn visit_expr(&mut self, expr: &ast::Expr) {
        if self.visit_receiver_expr(expr) {
            return;
        }
        if self.visit_operator_expr(expr) {
            return;
        }
        let _ = self.visit_collection_expr(expr);
    }

    fn visit_receiver_expr(&mut self, expr: &ast::Expr) -> bool {
        match expr {
            ast::Expr::Attribute(attr) => {
                self.track_field_access(attr);
                self.visit_expr(&attr.value);
            }
            ast::Expr::Call(call) => {
                self.track_method_call(call);
                self.visit_expr(&call.func);
                self.visit_expr_list(&call.arguments.args);
                for keyword in &call.arguments.keywords {
                    self.visit_expr(&keyword.value);
                }
            }
            _ => return false,
        }
        true
    }

    fn track_field_access(&mut self, attr: &ast::ExprAttribute) {
        let ast::Expr::Name(name) = &*attr.value else {
            return;
        };
        if self.is_receiver_name(name.id.as_str()) {
            self.used_fields.insert(attr.attr.id.to_string());
        }
    }

    fn track_method_call(&mut self, call: &ast::ExprCall) {
        let ast::Expr::Attribute(attr) = &*call.func else {
            return;
        };
        let ast::Expr::Name(name) = &*attr.value else {
            return;
        };
        if self.is_receiver_name(name.id.as_str()) {
            self.called_methods.insert(attr.attr.id.to_string());
        }
    }

    fn visit_operator_expr(&mut self, expr: &ast::Expr) -> bool {
        match expr {
            ast::Expr::BinOp(op) => {
                self.visit_expr(&op.left);
                self.visit_expr(&op.right);
            }
            ast::Expr::UnaryOp(op) => self.visit_expr(&op.operand),
            ast::Expr::BoolOp(op) => self.visit_expr_list(&op.values),
            ast::Expr::Compare(op) => {
                self.visit_expr(&op.left);
                self.visit_expr_list(&op.comparators);
            }
            ast::Expr::If(op) => {
                self.visit_expr(&op.test);
                self.visit_expr(&op.body);
                self.visit_expr(&op.orelse);
            }
            ast::Expr::Subscript(subscript) => {
                self.visit_expr(&subscript.value);
                self.visit_expr(&subscript.slice);
            }
            _ => return false,
        }
        true
    }

    fn visit_collection_expr(&mut self, expr: &ast::Expr) -> bool {
        match expr {
            ast::Expr::List(list) => self.visit_expr_list(&list.elts),
            ast::Expr::Tuple(tuple) => self.visit_expr_list(&tuple.elts),
            ast::Expr::Set(set) => self.visit_expr_list(&set.elts),
            ast::Expr::Dict(dict) => {
                for item in &dict.items {
                    if let Some(key) = &item.key {
                        self.visit_expr(key);
                    }
                    self.visit_expr(&item.value);
                }
            }
            _ => return false,
        }
        true
    }

    fn visit_expr_list(&mut self, exprs: &[ast::Expr]) {
        for expr in exprs {
            self.visit_expr(expr);
        }
    }
}
