use ruff_python_ast::{Expr, StmtImport, StmtImportFrom};
use rustc_hash::FxHashSet;

#[derive(Default)]
pub(super) struct TypeCheckingAliases {
    type_checking_names: FxHashSet<String>,
    typing_module_names: FxHashSet<String>,
}

impl TypeCheckingAliases {
    pub(super) fn record_import(&mut self, import_stmt: &StmtImport) {
        for alias in &import_stmt.names {
            let Some(top_level) = alias.name.split('.').next() else {
                continue;
            };
            if top_level == "typing" || top_level == "typing_extensions" {
                self.typing_module_names.insert(import_alias_name(alias));
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
        let module = module.as_ref();
        if module != "typing" && module != "typing_extensions" {
            return;
        }
        self.typing_module_names.insert(module.to_owned());
        for alias in &import_from.names {
            if alias.name.as_str() == "TYPE_CHECKING" {
                self.type_checking_names.insert(import_alias_name(alias));
            }
        }
    }

    fn is_type_checking_name(&self, name: &str) -> bool {
        self.type_checking_names.contains(name)
    }

    fn is_typing_module_name(&self, name: &str) -> bool {
        name == "typing" || name == "typing_extensions" || self.typing_module_names.contains(name)
    }
}

pub(super) fn is_type_checking_test(expr: &Expr, aliases: &TypeCheckingAliases) -> bool {
    match expr {
        Expr::Name(name) => aliases.is_type_checking_name(name.id.as_str()),
        Expr::Attribute(attr) if attr.attr.as_str() == "TYPE_CHECKING" => {
            if let Expr::Name(base) = &*attr.value {
                aliases.is_typing_module_name(base.id.as_str())
            } else {
                false
            }
        }
        _ => false,
    }
}

fn import_alias_name(alias: &ruff_python_ast::Alias) -> String {
    alias
        .asname
        .as_ref()
        .map_or_else(|| alias.name.to_string(), ToString::to_string)
}
