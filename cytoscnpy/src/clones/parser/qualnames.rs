use ruff_python_ast as ast;
use rustc_hash::FxHashMap;

/// Maps local names bound by module-level imports to their qualified origin,
/// so `from attrs import define` lets a bare `@define` resolve to
/// `attrs.define` and `import enum as e` lets `e.Enum` resolve to `enum.Enum`.
#[derive(Debug, Default)]
pub(super) struct ImportAliases {
    aliases: FxHashMap<String, String>,
}

impl ImportAliases {
    pub(super) fn from_module(body: &[ast::Stmt]) -> Self {
        let mut aliases = FxHashMap::default();
        for stmt in body {
            match stmt {
                ast::Stmt::Import(import) => {
                    for alias in &import.names {
                        let qualified = alias.name.to_string();
                        let local = alias.asname.as_ref().map_or_else(
                            || qualified.split('.').next().unwrap_or_default().to_owned(),
                            ToString::to_string,
                        );
                        let target = if alias.asname.is_some() {
                            qualified
                        } else {
                            local.clone()
                        };
                        aliases.insert(local, target);
                    }
                }
                ast::Stmt::ImportFrom(import) => {
                    let Some(module) = &import.module else {
                        continue;
                    };
                    for alias in &import.names {
                        let local = alias.asname.as_ref().unwrap_or(&alias.name).to_string();
                        aliases.insert(local, format!("{module}.{}", alias.name));
                    }
                }
                _ => {}
            }
        }
        Self { aliases }
    }

    /// Resolves a `Name`/`Attribute` chain (looking through `Call` and
    /// `Subscript`, e.g. `Protocol[T]` or `@dataclass(frozen=True)`) to a
    /// dotted path, substituting the import origin of the leading name.
    pub(super) fn resolve(&self, expr: &ast::Expr) -> Option<QualifiedName> {
        let path = match expr {
            ast::Expr::Name(name) => {
                let id = name.id.as_str();
                self.aliases
                    .get(id)
                    .cloned()
                    .unwrap_or_else(|| id.to_owned())
            }
            ast::Expr::Attribute(attr) => {
                let base = self.resolve(&attr.value)?;
                format!("{}.{}", base.path, attr.attr)
            }
            ast::Expr::Call(call) => return self.resolve(&call.func),
            ast::Expr::Subscript(sub) => return self.resolve(&sub.value),
            _ => return None,
        };
        Some(QualifiedName { path })
    }
}

/// A dotted name such as `enum.Enum`, `attrs.define` or a bare `BaseEnum`.
pub(super) struct QualifiedName {
    path: String,
}

impl QualifiedName {
    /// Final segment, e.g. `Enum` for `enum.Enum`.
    pub(super) fn name(&self) -> &str {
        self.path.rsplit('.').next().unwrap_or(&self.path)
    }

    /// Leading segment, e.g. `django` for `django.db.models.Model`.
    pub(super) fn root(&self) -> &str {
        self.path.split('.').next().unwrap_or(&self.path)
    }

    /// True for an unqualified name that no import resolved.
    pub(super) fn is_bare(&self) -> bool {
        !self.path.contains('.')
    }

    /// True when the leading module is one of `roots`.
    pub(super) fn is_from(&self, roots: &[&str]) -> bool {
        !self.is_bare() && roots.contains(&self.root())
    }
}
