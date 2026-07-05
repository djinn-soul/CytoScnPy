use super::imports_dynamic::collect_dynamic_imports_from_stmt;
use super::imports_type_checking::{is_type_checking_test, TypeCheckingAliases};
use super::{ImportOccurrence, ImportScan};
use crate::utils::LineIndex;
use ruff_python_ast::{self as ast, Stmt};
use ruff_python_parser::parse_module;
use ruff_text_size::Ranged;
use rustc_hash::FxHashSet;

pub(super) fn extract_imports_from_file(file: &std::path::Path, is_production: bool) -> ImportScan {
    let mut scan = ImportScan {
        all: FxHashSet::default(),
        production: FxHashSet::default(),
        occurrences: Vec::new(),
    };
    if let Ok(content) = std::fs::read_to_string(file) {
        if let Ok(parsed) = parse_module(&content) {
            let line_index = LineIndex::new(&content);
            let mut aliases = TypeCheckingAliases::default();
            collect_imports(
                &parsed.into_syntax().body,
                &mut scan,
                &mut aliases,
                file,
                &line_index,
                is_production,
                false,
            );
            if is_production {
                scan.production.extend(scan.all.iter().cloned());
            }
        }
    }
    scan
}

fn collect_imports(
    stmts: &[Stmt],
    scan: &mut ImportScan,
    aliases: &mut TypeCheckingAliases,
    file: &std::path::Path,
    line_index: &LineIndex,
    is_production: bool,
    in_type_checking_block: bool,
) {
    for stmt in stmts {
        if !in_type_checking_block {
            collect_dynamic_imports_from_stmt(stmt, scan, file, line_index, is_production);
        }
        match stmt {
            Stmt::Import(import_stmt) => {
                aliases.record_import(import_stmt);
                if !in_type_checking_block {
                    for alias in &import_stmt.names {
                        if let Some(top_level) = alias.name.split('.').next() {
                            add_import_occurrence(
                                scan,
                                top_level,
                                stmt,
                                file,
                                line_index,
                                is_production,
                            );
                        }
                    }
                }
            }
            Stmt::ImportFrom(import_from) => {
                aliases.record_import_from(import_from);
                if !in_type_checking_block && import_from.level == 0 {
                    if let Some(module) = &import_from.module {
                        if let Some(top_level) = module.as_ref().split('.').next() {
                            add_import_occurrence(
                                scan,
                                top_level,
                                stmt,
                                file,
                                line_index,
                                is_production,
                            );
                        }
                    }
                }
            }
            Stmt::FunctionDef(f) => {
                collect_imports(
                    &f.body,
                    scan,
                    aliases,
                    file,
                    line_index,
                    is_production,
                    in_type_checking_block,
                );
            }
            Stmt::ClassDef(c) => {
                collect_imports(
                    &c.body,
                    scan,
                    aliases,
                    file,
                    line_index,
                    is_production,
                    in_type_checking_block,
                );
            }
            Stmt::If(i) => {
                let guarded = is_type_checking_test(&i.test, aliases);
                collect_imports(
                    &i.body,
                    scan,
                    aliases,
                    file,
                    line_index,
                    is_production,
                    in_type_checking_block || guarded,
                );
                for clause in &i.elif_else_clauses {
                    collect_imports(
                        &clause.body,
                        scan,
                        aliases,
                        file,
                        line_index,
                        is_production,
                        in_type_checking_block,
                    );
                }
            }
            Stmt::For(f) => {
                collect_imports(
                    &f.body,
                    scan,
                    aliases,
                    file,
                    line_index,
                    is_production,
                    in_type_checking_block,
                );
                collect_imports(
                    &f.orelse,
                    scan,
                    aliases,
                    file,
                    line_index,
                    is_production,
                    in_type_checking_block,
                );
            }
            Stmt::While(w) => {
                collect_imports(
                    &w.body,
                    scan,
                    aliases,
                    file,
                    line_index,
                    is_production,
                    in_type_checking_block,
                );
                collect_imports(
                    &w.orelse,
                    scan,
                    aliases,
                    file,
                    line_index,
                    is_production,
                    in_type_checking_block,
                );
            }
            Stmt::With(w) => {
                collect_imports(
                    &w.body,
                    scan,
                    aliases,
                    file,
                    line_index,
                    is_production,
                    in_type_checking_block,
                );
            }
            Stmt::Try(t) => collect_try_imports(
                t,
                scan,
                aliases,
                file,
                line_index,
                is_production,
                in_type_checking_block,
            ),
            Stmt::Match(m) => {
                for case in &m.cases {
                    collect_imports(
                        &case.body,
                        scan,
                        aliases,
                        file,
                        line_index,
                        is_production,
                        in_type_checking_block,
                    );
                }
            }
            _ => {}
        }
    }
}

fn collect_try_imports(
    node: &ast::StmtTry,
    scan: &mut ImportScan,
    aliases: &mut TypeCheckingAliases,
    file: &std::path::Path,
    line_index: &LineIndex,
    is_production: bool,
    in_type_checking_block: bool,
) {
    collect_imports(
        &node.body,
        scan,
        aliases,
        file,
        line_index,
        is_production,
        in_type_checking_block,
    );
    for handler in &node.handlers {
        let ast::ExceptHandler::ExceptHandler(handler) = handler;
        collect_imports(
            &handler.body,
            scan,
            aliases,
            file,
            line_index,
            is_production,
            in_type_checking_block,
        );
    }
    collect_imports(
        &node.orelse,
        scan,
        aliases,
        file,
        line_index,
        is_production,
        in_type_checking_block,
    );
    collect_imports(
        &node.finalbody,
        scan,
        aliases,
        file,
        line_index,
        is_production,
        in_type_checking_block,
    );
}

fn add_import_occurrence(
    scan: &mut ImportScan,
    name: &str,
    stmt: &Stmt,
    file: &std::path::Path,
    line_index: &LineIndex,
    is_production: bool,
) {
    scan.all.insert(name.to_owned());
    scan.occurrences.push(ImportOccurrence {
        name: name.to_owned(),
        file: file.to_path_buf(),
        line: line_index.line_index(stmt.range().start()),
        column: line_index.column_index(stmt.range().start()),
        is_production,
    });
}
