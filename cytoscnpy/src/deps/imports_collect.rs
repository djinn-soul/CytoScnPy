use super::imports_dynamic::{collect_dynamic_imports_from_stmt, DynamicImportAliases};
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
        type_checking: FxHashSet::default(),
        occurrences: Vec::new(),
    };
    if let Ok(content) = std::fs::read_to_string(file) {
        if let Ok(parsed) = parse_module(&content) {
            let line_index = LineIndex::new(&content);
            let mut aliases = TypeCheckingAliases::default();
            let mut dynamic_aliases = DynamicImportAliases::default();
            collect_imports(
                &parsed.into_syntax().body,
                &mut scan,
                &mut aliases,
                &mut dynamic_aliases,
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
    dynamic_aliases: &mut DynamicImportAliases,
    file: &std::path::Path,
    line_index: &LineIndex,
    is_production: bool,
    in_type_checking_block: bool,
) {
    for stmt in stmts {
        if !in_type_checking_block {
            collect_dynamic_imports_from_stmt(
                stmt,
                scan,
                dynamic_aliases,
                file,
                line_index,
                is_production,
            );
        }
        match stmt {
            Stmt::Import(import_stmt) => {
                aliases.record_import(import_stmt);
                let names = import_stmt
                    .names
                    .iter()
                    .filter_map(|alias| alias.name.split('.').next());
                if in_type_checking_block {
                    scan.type_checking
                        .extend(names.map(std::borrow::ToOwned::to_owned));
                } else {
                    dynamic_aliases.record_import(import_stmt);
                    let mut recorded = FxHashSet::default();
                    for top_level in names {
                        if recorded.insert(top_level) {
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
            // Only absolute imports (level 0) can name a distribution; relative
            // imports (`from . import x`) are handled by the arm below.
            Stmt::ImportFrom(import_from) if import_from.level == 0 => {
                aliases.record_import_from(import_from);
                let Some(top_level) = import_from
                    .module
                    .as_ref()
                    .and_then(|module| module.as_ref().split('.').next())
                else {
                    continue;
                };
                if in_type_checking_block {
                    scan.type_checking.insert(top_level.to_owned());
                } else {
                    dynamic_aliases.record_import_from(import_from);
                    add_import_occurrence(scan, top_level, stmt, file, line_index, is_production);
                }
            }
            Stmt::ImportFrom(import_from) => aliases.record_import_from(import_from),
            Stmt::FunctionDef(f) => {
                collect_imports(
                    &f.body,
                    scan,
                    aliases,
                    dynamic_aliases,
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
                    dynamic_aliases,
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
                    dynamic_aliases,
                    file,
                    line_index,
                    is_production,
                    in_type_checking_block || guarded,
                );
                for clause in &i.elif_else_clauses {
                    let guarded = clause
                        .test
                        .as_ref()
                        .is_some_and(|test| is_type_checking_test(test, aliases));
                    collect_imports(
                        &clause.body,
                        scan,
                        aliases,
                        dynamic_aliases,
                        file,
                        line_index,
                        is_production,
                        in_type_checking_block || guarded,
                    );
                }
            }
            Stmt::For(f) => {
                collect_imports(
                    &f.body,
                    scan,
                    aliases,
                    dynamic_aliases,
                    file,
                    line_index,
                    is_production,
                    in_type_checking_block,
                );
                collect_imports(
                    &f.orelse,
                    scan,
                    aliases,
                    dynamic_aliases,
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
                    dynamic_aliases,
                    file,
                    line_index,
                    is_production,
                    in_type_checking_block,
                );
                collect_imports(
                    &w.orelse,
                    scan,
                    aliases,
                    dynamic_aliases,
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
                    dynamic_aliases,
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
                dynamic_aliases,
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
                        dynamic_aliases,
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
    dynamic_aliases: &mut DynamicImportAliases,
    file: &std::path::Path,
    line_index: &LineIndex,
    is_production: bool,
    in_type_checking_block: bool,
) {
    collect_imports(
        &node.body,
        scan,
        aliases,
        dynamic_aliases,
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
            dynamic_aliases,
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
        dynamic_aliases,
        file,
        line_index,
        is_production,
        in_type_checking_block,
    );
    collect_imports(
        &node.finalbody,
        scan,
        aliases,
        dynamic_aliases,
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
