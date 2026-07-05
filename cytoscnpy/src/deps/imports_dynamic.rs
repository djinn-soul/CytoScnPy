use super::{ImportOccurrence, ImportScan};
use crate::utils::LineIndex;
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::Ranged;

pub(super) fn collect_dynamic_imports_from_stmt(
    stmt: &Stmt,
    scan: &mut ImportScan,
    file: &std::path::Path,
    line_index: &LineIndex,
    is_production: bool,
) {
    let mut collector = DynamicImportCollector {
        scan,
        file,
        line_index,
        is_production,
    };
    match stmt {
        Stmt::Assign(node) => collector.visit_expr(&node.value),
        Stmt::AnnAssign(node) => {
            if let Some(value) = &node.value {
                collector.visit_expr(value);
            }
        }
        Stmt::AugAssign(node) => collector.visit_expr(&node.value),
        Stmt::Expr(node) => collector.visit_expr(&node.value),
        Stmt::Return(node) => {
            if let Some(value) = &node.value {
                collector.visit_expr(value);
            }
        }
        Stmt::If(node) => collector.visit_expr(&node.test),
        Stmt::While(node) => collector.visit_expr(&node.test),
        Stmt::For(node) => collector.visit_expr(&node.iter),
        Stmt::With(node) => {
            for item in &node.items {
                collector.visit_expr(&item.context_expr);
            }
        }
        Stmt::Match(node) => collector.visit_expr(&node.subject),
        Stmt::Raise(node) => {
            if let Some(exc) = &node.exc {
                collector.visit_expr(exc);
            }
            if let Some(cause) = &node.cause {
                collector.visit_expr(cause);
            }
        }
        Stmt::Assert(node) => {
            collector.visit_expr(&node.test);
            if let Some(msg) = &node.msg {
                collector.visit_expr(msg);
            }
        }
        _ => {}
    }
}

struct DynamicImportCollector<'a> {
    scan: &'a mut ImportScan,
    file: &'a std::path::Path,
    line_index: &'a LineIndex,
    is_production: bool,
}

impl<'a> Visitor<'a> for DynamicImportCollector<'_> {
    fn visit_expr(&mut self, expr: &'a Expr) {
        if let Expr::Call(call) = expr {
            self.collect_from_call(call);
        }
        visitor::walk_expr(self, expr);
    }
}

impl DynamicImportCollector<'_> {
    fn collect_from_call(&mut self, call: &ast::ExprCall) {
        let Some(module_name) = dynamic_import_module_name(call) else {
            return;
        };
        let Some(top_level) = module_name.split('.').next() else {
            return;
        };
        self.scan.all.insert(top_level.to_owned());
        self.scan.occurrences.push(ImportOccurrence {
            name: top_level.to_owned(),
            file: self.file.to_path_buf(),
            line: self.line_index.line_index(call.range().start()),
            column: self.line_index.column_index(call.range().start()),
            is_production: self.is_production,
        });
    }
}

fn dynamic_import_module_name(call: &ast::ExprCall) -> Option<String> {
    let first_arg = call.arguments.args.first()?;
    match &*call.func {
        Expr::Name(name) if name.id.as_str() == "__import__" => string_literal_value(first_arg),
        Expr::Attribute(attr) if attr.attr.as_str() == "import_module" => match &*attr.value {
            Expr::Name(name) if name.id.as_str() == "importlib" => string_literal_value(first_arg),
            _ => None,
        },
        _ => None,
    }
}

fn string_literal_value(expr: &Expr) -> Option<String> {
    match expr {
        Expr::StringLiteral(value) => Some(value.value.to_string()),
        _ => None,
    }
}
