use super::{ImportOccurrence, ImportScan};
use crate::utils::LineIndex;
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, Stmt, StmtImport, StmtImportFrom};
use ruff_text_size::Ranged;
use rustc_hash::FxHashSet;

#[derive(Default)]
pub(super) struct DynamicImportAliases {
    importlib_module_names: FxHashSet<String>,
    import_module_names: FxHashSet<String>,
}

impl DynamicImportAliases {
    pub(super) fn record_import(&mut self, import_stmt: &StmtImport) {
        for alias in &import_stmt.names {
            if alias.name.as_str() == "importlib" {
                self.importlib_module_names.insert(import_alias_name(alias));
            }
        }
    }

    pub(super) fn record_import_from(&mut self, import_from: &StmtImportFrom) {
        if import_from.level > 0 {
            return;
        }
        let Some(module) = &import_from.module else {
            return;
        };
        if module.as_ref() != "importlib" {
            return;
        }
        for alias in &import_from.names {
            if alias.name.as_str() == "import_module" {
                self.import_module_names.insert(import_alias_name(alias));
            }
        }
    }

    fn is_importlib_module_name(&self, name: &str) -> bool {
        name == "importlib" || self.importlib_module_names.contains(name)
    }

    fn is_import_module_name(&self, name: &str) -> bool {
        self.import_module_names.contains(name)
    }
}

pub(super) fn collect_dynamic_imports_from_stmt(
    stmt: &Stmt,
    scan: &mut ImportScan,
    aliases: &DynamicImportAliases,
    file: &std::path::Path,
    line_index: &LineIndex,
    is_production: bool,
) {
    let mut collector = DynamicImportCollector {
        scan,
        aliases,
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
    aliases: &'a DynamicImportAliases,
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
        let Some(module_name) = dynamic_import_module_name(call, self.aliases) else {
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

fn dynamic_import_module_name(
    call: &ast::ExprCall,
    aliases: &DynamicImportAliases,
) -> Option<String> {
    let first_arg = call.arguments.args.first()?;
    match &*call.func {
        Expr::Name(name) if name.id.as_str() == "__import__" => string_literal_value(first_arg),
        Expr::Name(name) if aliases.is_import_module_name(name.id.as_str()) => {
            string_literal_value(first_arg)
        }
        Expr::Attribute(attr) if attr.attr.as_str() == "import_module" => match &*attr.value {
            Expr::Name(name) if aliases.is_importlib_module_name(name.id.as_str()) => {
                string_literal_value(first_arg)
            }
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

fn import_alias_name(alias: &ruff_python_ast::Alias) -> String {
    alias
        .asname
        .as_ref()
        .map_or_else(|| alias.name.to_string(), ToString::to_string)
}
