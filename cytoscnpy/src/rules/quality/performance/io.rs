use super::{
    collect_name_targets, create_finding, is_scope_boundary, ScopedNames, META_FILE_READ_RISK,
    META_PANDAS_CHUNK_RISK,
};
use crate::rules::{Context, Finding, Rule, RuleMetadata};
use ruff_python_ast::Expr;
use ruff_text_size::Ranged;

pub(in crate::rules::quality) struct FileReadMemoryRiskRule {
    buffers: ScopedNames,
}
impl FileReadMemoryRiskRule {
    pub fn new() -> Self {
        Self {
            buffers: ScopedNames::new(),
        }
    }

    fn update_buffers(&mut self, target: &Expr, value: &Expr) {
        let mut names = Vec::new();
        collect_name_targets(target, &mut names);
        if names.is_empty() {
            return;
        }
        let is_buffer = is_buffer_expr(value);
        for name in names {
            if is_buffer {
                self.buffers.insert(name);
            } else {
                self.buffers.remove(&name);
            }
        }
    }
}
impl Rule for FileReadMemoryRiskRule {
    fn name(&self) -> &'static str {
        "FileReadMemoryRiskRule"
    }
    fn metadata(&self) -> RuleMetadata {
        META_FILE_READ_RISK
    }
    fn enter_stmt(
        &mut self,
        stmt: &ruff_python_ast::Stmt,
        _context: &Context,
    ) -> Option<Vec<Finding>> {
        if is_scope_boundary(stmt) {
            self.buffers.push_scope();
        }
        match stmt {
            ruff_python_ast::Stmt::Assign(assign) => {
                for target in &assign.targets {
                    self.update_buffers(target, &assign.value);
                }
            }
            ruff_python_ast::Stmt::AnnAssign(assign) => {
                if let Some(value) = &assign.value {
                    self.update_buffers(&assign.target, value);
                }
            }
            _ => {}
        }
        None
    }
    fn leave_stmt(
        &mut self,
        stmt: &ruff_python_ast::Stmt,
        _context: &Context,
    ) -> Option<Vec<Finding>> {
        if is_scope_boundary(stmt) {
            self.buffers.pop_scope();
        }
        None
    }
    fn visit_expr(&mut self, expr: &Expr, context: &Context) -> Option<Vec<Finding>> {
        if let Expr::Call(call) = expr {
            if let Expr::Attribute(attr) = &*call.func {
                let method_name = attr.attr.as_str();
                if method_name == "read" || method_name == "readlines" {
                    if !call.arguments.args.is_empty() || !call.arguments.keywords.is_empty() {
                        return None;
                    }
                    if is_in_memory_buffer(&attr.value, &self.buffers) {
                        return None;
                    }
                    return Some(vec![create_finding(
                        &format!("Potential Memory Risk: '{method_name}()' loads entire file into RAM. Consider iterating line-by-line."),
                        META_FILE_READ_RISK,
                        context,
                        expr.range().start(),
                        "MEDIUM",
                    )]);
                }
            }
        }
        None
    }
}

pub(in crate::rules::quality) struct PandasChunksizeRiskRule;
impl Rule for PandasChunksizeRiskRule {
    fn name(&self) -> &'static str {
        "PandasChunksizeRiskRule"
    }
    fn metadata(&self) -> RuleMetadata {
        META_PANDAS_CHUNK_RISK
    }
    fn visit_expr(&mut self, expr: &Expr, context: &Context) -> Option<Vec<Finding>> {
        if let Expr::Call(call) = expr {
            if let Expr::Attribute(attr) = &*call.func {
                if attr.attr.as_str() == "read_csv" {
                    // Check if 'chunksize' is in keywords
                    let has_chunksize = call.arguments.keywords.iter().any(|kw| {
                        kw.arg
                            .as_ref()
                            .is_some_and(|arg| arg.as_str() == "chunksize")
                    });
                    let has_nrows = call
                        .arguments
                        .keywords
                        .iter()
                        .any(|kw| kw.arg.as_ref().is_some_and(|arg| arg.as_str() == "nrows"));
                    let has_iterator = call.arguments.keywords.iter().any(|kw| {
                        kw.arg
                            .as_ref()
                            .is_some_and(|arg| arg.as_str() == "iterator")
                            && matches!(&kw.value, Expr::BooleanLiteral(b) if b.value)
                    });

                    if !has_chunksize && !has_nrows && !has_iterator {
                        return Some(vec![create_finding(
                            "Pandas Memory Risk: read_csv used without 'chunksize'. Large files may crash RAM.",
                            META_PANDAS_CHUNK_RISK,
                            context,
                            expr.range().start(),
                            "LOW",
                        )]);
                    }
                }
            }
        }
        None
    }
}

fn is_in_memory_buffer(expr: &Expr, buffers: &ScopedNames) -> bool {
    match expr {
        Expr::Name(name) => buffers.contains(name.id.as_str()),
        Expr::Call(call) => match &*call.func {
            Expr::Attribute(attr) => {
                matches!(attr.attr.as_str(), "StringIO" | "BytesIO")
                    && matches!(&*attr.value, Expr::Name(name) if name.id.as_str() == "io")
            }
            Expr::Name(name) => matches!(name.id.as_str(), "StringIO" | "BytesIO"),
            _ => false,
        },
        _ => false,
    }
}

fn is_buffer_expr(expr: &Expr) -> bool {
    let Expr::Call(call) = expr else {
        return false;
    };
    match &*call.func {
        Expr::Attribute(attr) => {
            matches!(attr.attr.as_str(), "StringIO" | "BytesIO")
                && matches!(&*attr.value, Expr::Name(name) if name.id.as_str() == "io")
        }
        Expr::Name(name) => matches!(name.id.as_str(), "StringIO" | "BytesIO"),
        _ => false,
    }
}
