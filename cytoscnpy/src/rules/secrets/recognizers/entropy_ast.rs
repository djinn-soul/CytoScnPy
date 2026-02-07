use super::entropy::EntropyRecognizer;
use super::types::{is_test_name, RawFinding};
use crate::utils::LineIndex;
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::Ranged;

/// Context for AST traversal.
#[derive(Clone, Copy, Default)]
struct Context {
    in_logging: bool,
    safe_assignment: bool,
}

impl EntropyRecognizer {
    /// Check if an expression is a call to logging/print functions.
    fn is_logging_call(expr: &Expr) -> bool {
        if let Expr::Call(call) = expr {
            match &*call.func {
                Expr::Attribute(attr) => {
                    let attr_name = attr.attr.as_str();
                    // Common logging methods
                    if matches!(
                        attr_name,
                        "debug"
                            | "info"
                            | "warning"
                            | "warn"
                            | "error"
                            | "critical"
                            | "log"
                            | "exception"
                    ) {
                        // Check if base is 'logger' or 'logging' or 'log' or 'self.logger'
                        if let Expr::Name(name) = &*attr.value {
                            let id = name.id.as_str();
                            return matches!(id, "logger" | "logging" | "log");
                        }
                        // Handle self.logger.info
                        if let Expr::Attribute(inner_attr) = &*attr.value {
                            if inner_attr.attr.as_str() == "logger" {
                                return true;
                            }
                        }
                    }
                    false
                }
                Expr::Name(name) => {
                    // check for global 'print' or 'log'
                    matches!(name.id.as_str(), "print" | "log")
                }
                _ => false,
            }
        } else {
            false
        }
    }

    fn visit_expr(
        &self,
        expr: &Expr,
        line_index: &LineIndex,
        findings: &mut Vec<RawFinding>,
        ctx: Context,
    ) {
        match expr {
            Expr::StringLiteral(s) => {
                if !ctx.in_logging && !ctx.safe_assignment {
                    self.check_string(
                        &s.value.to_string(),
                        line_index.line_index(expr.start()),
                        findings,
                    );
                }
            }
            Expr::Call(call) => {
                let is_log = Self::is_logging_call(expr);
                let new_ctx = Context {
                    in_logging: ctx.in_logging || is_log,
                    ..ctx
                };

                self.visit_expr(&call.func, line_index, findings, ctx);

                for arg in &call.arguments.args {
                    self.visit_expr(arg, line_index, findings, new_ctx);
                }
                for kw in &call.arguments.keywords {
                    self.visit_expr(&kw.value, line_index, findings, new_ctx);
                }
            }
            Expr::List(l) => {
                for e in &l.elts {
                    self.visit_expr(e, line_index, findings, ctx);
                }
            }
            Expr::Tuple(t) => {
                for e in &t.elts {
                    self.visit_expr(e, line_index, findings, ctx);
                }
            }
            Expr::Set(s) => {
                for e in &s.elts {
                    self.visit_expr(e, line_index, findings, ctx);
                }
            }
            Expr::Dict(d) => {
                for item in &d.items {
                    if let Some(key) = &item.key {
                        self.visit_expr(key, line_index, findings, ctx);
                    }
                    self.visit_expr(&item.value, line_index, findings, ctx);
                }
            }
            Expr::BinOp(b) => {
                self.visit_expr(&b.left, line_index, findings, ctx);
                self.visit_expr(&b.right, line_index, findings, ctx);
            }
            Expr::UnaryOp(u) => {
                self.visit_expr(&u.operand, line_index, findings, ctx);
            }
            Expr::BoolOp(b) => {
                for v in &b.values {
                    self.visit_expr(v, line_index, findings, ctx);
                }
            }
            Expr::Attribute(a) => {
                self.visit_expr(&a.value, line_index, findings, ctx);
            }
            Expr::Subscript(s) => {
                self.visit_expr(&s.value, line_index, findings, ctx);
                self.visit_expr(&s.slice, line_index, findings, ctx);
            }
            Expr::Await(a) => self.visit_expr(&a.value, line_index, findings, ctx),
            Expr::Yield(y) => {
                if let Some(v) = &y.value {
                    self.visit_expr(v, line_index, findings, ctx);
                }
            }
            Expr::YieldFrom(y) => self.visit_expr(&y.value, line_index, findings, ctx),
            Expr::Compare(c) => {
                self.visit_expr(&c.left, line_index, findings, ctx);
                for comparator in &c.comparators {
                    self.visit_expr(comparator, line_index, findings, ctx);
                }
            }
            _ => {}
        }
    }

    fn check_target_safety(expr: &Expr) -> bool {
        if let Expr::Name(name) = expr {
            let lower = name.id.as_str().to_lowercase();
            if lower.contains("public")
                || lower.contains("example")
                || lower.contains("sample")
                || is_test_name(&lower)
                || lower.ends_with("_regex")
                || lower.ends_with("_pattern")
                || lower.ends_with("_re")
                || lower.ends_with("_fmt")
                || lower.ends_with("_format")
            {
                return true;
            }
            if lower.contains("jwt") && lower.contains("token") {
                return true;
            }
        }
        false
    }

    pub(super) fn visit_stmts(
        &self,
        stmts: &[Stmt],
        line_index: &LineIndex,
        findings: &mut Vec<RawFinding>,
    ) {
        let default_ctx = Context::default();
        for stmt in stmts {
            match stmt {
                Stmt::Assign(a) => {
                    let mut safe = false;
                    for t in &a.targets {
                        self.visit_expr(t, line_index, findings, default_ctx);
                        if Self::check_target_safety(t) {
                            safe = true;
                        }
                    }
                    let ctx = Context {
                        safe_assignment: safe,
                        ..default_ctx
                    };
                    self.visit_expr(&a.value, line_index, findings, ctx);
                }
                Stmt::AnnAssign(a) => {
                    self.visit_expr(&a.target, line_index, findings, default_ctx);
                    let safe = Self::check_target_safety(&a.target);
                    if let Some(val) = &a.value {
                        let ctx = Context {
                            safe_assignment: safe,
                            ..default_ctx
                        };
                        self.visit_expr(val, line_index, findings, ctx);
                    }
                }
                Stmt::Expr(e) => self.visit_expr(&e.value, line_index, findings, default_ctx),
                Stmt::Return(r) => {
                    if let Some(v) = r.value.as_deref() {
                        self.visit_expr(v, line_index, findings, default_ctx);
                    }
                }
                Stmt::FunctionDef(f) => {
                    // Visit decorators (e.g., @auth(token="secret"))
                    for dec in &f.decorator_list {
                        self.visit_expr(&dec.expression, line_index, findings, default_ctx);
                    }
                    // Visit parameter defaults (e.g., def login(token="sk_live_..."))
                    // ruff uses ParameterWithDefault with .default field per parameter
                    for arg in &f.parameters.posonlyargs {
                        if let Some(default) = &arg.default {
                            self.visit_expr(default, line_index, findings, default_ctx);
                        }
                    }
                    for arg in &f.parameters.args {
                        if let Some(default) = &arg.default {
                            self.visit_expr(default, line_index, findings, default_ctx);
                        }
                    }
                    for arg in &f.parameters.kwonlyargs {
                        if let Some(default) = &arg.default {
                            self.visit_expr(default, line_index, findings, default_ctx);
                        }
                    }
                    // Visit return annotation (unlikely to contain secrets but complete)
                    if let Some(ret) = &f.returns {
                        self.visit_expr(ret, line_index, findings, default_ctx);
                    }
                    // Visit body
                    self.visit_stmts(&f.body, line_index, findings);
                }
                Stmt::ClassDef(c) => self.visit_stmts(&c.body, line_index, findings),
                Stmt::If(i) => {
                    self.visit_expr(&i.test, line_index, findings, default_ctx);
                    self.visit_stmts(&i.body, line_index, findings);
                    for clause in &i.elif_else_clauses {
                        self.visit_stmts(&clause.body, line_index, findings);
                    }
                }
                Stmt::For(f) => {
                    self.visit_expr(&f.target, line_index, findings, default_ctx);
                    self.visit_expr(&f.iter, line_index, findings, default_ctx);
                    self.visit_stmts(&f.body, line_index, findings);
                    self.visit_stmts(&f.orelse, line_index, findings);
                }
                Stmt::While(w) => {
                    self.visit_expr(&w.test, line_index, findings, default_ctx);
                    self.visit_stmts(&w.body, line_index, findings);
                    self.visit_stmts(&w.orelse, line_index, findings);
                }
                Stmt::Try(t) => {
                    self.visit_stmts(&t.body, line_index, findings);
                    for handler in &t.handlers {
                        #[allow(irrefutable_let_patterns)]
                        if let ast::ExceptHandler::ExceptHandler(h) = handler {
                            self.visit_stmts(&h.body, line_index, findings);
                        }
                    }
                    self.visit_stmts(&t.orelse, line_index, findings);
                    self.visit_stmts(&t.finalbody, line_index, findings);
                }
                Stmt::With(w) => {
                    for item in &w.items {
                        self.visit_expr(&item.context_expr, line_index, findings, default_ctx);
                    }
                    self.visit_stmts(&w.body, line_index, findings);
                }
                Stmt::Match(m) => {
                    self.visit_expr(&m.subject, line_index, findings, default_ctx);
                    for case in &m.cases {
                        self.visit_stmts(&case.body, line_index, findings);
                    }
                }
                Stmt::AugAssign(a) => {
                    self.visit_expr(&a.target, line_index, findings, default_ctx);
                    self.visit_expr(&a.value, line_index, findings, default_ctx);
                }
                Stmt::Delete(d) => {
                    for t in &d.targets {
                        self.visit_expr(t, line_index, findings, default_ctx);
                    }
                }
                Stmt::Raise(r) => {
                    if let Some(exc) = &r.exc {
                        self.visit_expr(exc, line_index, findings, default_ctx);
                    }
                    if let Some(cause) = &r.cause {
                        self.visit_expr(cause, line_index, findings, default_ctx);
                    }
                }
                Stmt::Assert(a) => {
                    self.visit_expr(&a.test, line_index, findings, default_ctx);
                    if let Some(msg) = &a.msg {
                        self.visit_expr(msg, line_index, findings, default_ctx);
                    }
                }
                _ => {}
            }
        }
    }
}
