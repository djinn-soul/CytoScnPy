//! Tracks typing module imports and `TYPE_CHECKING` guards.

use ruff_python_ast::{Expr, StmtImport, StmtImportFrom};
use rustc_hash::FxHashSet;

#[derive(Default)]
pub(super) struct TypeCheckingState {
    type_checking_names: FxHashSet<String>,
    typing_modules: FxHashSet<String>,
}

impl TypeCheckingState {
    pub(super) fn record_import(&mut self, import_stmt: &StmtImport) {
        for alias in &import_stmt.names {
            let Some(top) = alias.name.split('.').next() else {
                continue;
            };
            if top == "typing" || top == "typing_extensions" {
                let alias_name = alias
                    .asname
                    .as_ref()
                    .map_or_else(|| alias.name.to_string(), ToString::to_string);
                self.typing_modules.insert(alias_name);
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
        let module_str = module.as_ref();
        if module_str != "typing" && module_str != "typing_extensions" {
            return;
        }
        self.typing_modules.insert(module_str.to_owned());
        for alias in &import_from.names {
            if alias.name.as_str() == "TYPE_CHECKING" {
                let alias_name = alias
                    .asname
                    .as_ref()
                    .map_or_else(|| alias.name.to_string(), ToString::to_string);
                self.type_checking_names.insert(alias_name);
            }
        }
    }

    pub(super) fn is_type_checking_test(&self, expr: &Expr) -> bool {
        match expr {
            Expr::Name(name) => self.type_checking_names.contains(name.id.as_str()),
            Expr::Attribute(attr) if attr.attr.as_str() == "TYPE_CHECKING" => {
                if let Expr::Name(base) = &*attr.value {
                    let base_name = base.id.as_str();
                    base_name == "typing"
                        || base_name == "typing_extensions"
                        || self.typing_modules.contains(base_name)
                } else {
                    false
                }
            }
            _ => false,
        }
    }
}
