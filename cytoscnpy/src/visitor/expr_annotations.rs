//! Annotation, type-parameter, and generic-type handling for the visitor.

use super::{ast, CytoScnPyVisitor, Expr};

impl CytoScnPyVisitor<'_> {
    /// Visits a type annotation. String literals inside it are treated as
    /// forward references and parsed, so names used only there count as used.
    pub(super) fn visit_annotation(&mut self, expr: &Expr) {
        let prev = std::mem::replace(&mut self.in_annotation, true);
        self.visit_expr(expr);
        self.in_annotation = prev;
    }

    /// Visits a non-type expression nested inside an annotation, such as the
    /// values of `Literal[...]` or the metadata of `Annotated[...]`.
    pub(super) fn visit_non_annotation(&mut self, expr: &Expr) {
        let prev = std::mem::replace(&mut self.in_annotation, false);
        self.visit_expr(expr);
        self.in_annotation = prev;
    }

    /// Visits PEP 695 type parameter bounds and defaults.
    pub(super) fn visit_type_params(&mut self, type_params: Option<&ast::TypeParams>) {
        let Some(type_params) = type_params else {
            return;
        };
        for param in &**type_params {
            let (bound, default) = match param {
                ast::TypeParam::TypeVar(tv) => (tv.bound.as_deref(), tv.default.as_deref()),
                ast::TypeParam::ParamSpec(ps) => (None, ps.default.as_deref()),
                ast::TypeParam::TypeVarTuple(tvt) => (None, tvt.default.as_deref()),
            };
            for expr in bound.into_iter().chain(default) {
                self.visit_annotation(expr);
            }
        }
    }

    /// Visits a subscript in an annotation, keeping `Literal[...]` values and
    /// `Annotated[...]` metadata out of annotation context.
    pub(super) fn visit_annotation_subscript(&mut self, node: &ast::ExprSubscript) {
        self.visit_expr(&node.value);
        let special = match &*node.value {
            Expr::Name(name) => name.id.as_str(),
            Expr::Attribute(attr) => attr.attr.as_str(),
            _ => "",
        };
        match (special, &*node.slice) {
            ("Literal", slice) => self.visit_non_annotation(slice),
            ("Annotated", Expr::Tuple(tuple)) => {
                if let Some((first, metadata)) = tuple.elts.split_first() {
                    self.visit_expr(first);
                    for expr in metadata {
                        self.visit_non_annotation(expr);
                    }
                }
            }
            (_, slice) => self.visit_expr(slice),
        }
    }

    /// Visits string arguments of `cast(...)`, `TypeVar(...)`, `NewType(...)` and
    /// the functional `TypedDict(...)`/`NamedTuple(...)` forms that are type
    /// expressions (forward references).
    pub(super) fn visit_typing_call_string_args(&mut self, name: &str, node: &ast::ExprCall) {
        let args = &node.arguments.args;
        let mut type_exprs: Vec<&Expr> = match name {
            "cast" => args.iter().take(1).collect(),
            "TypeVar" | "NewType" => args.iter().skip(1).collect(),
            // TypedDict("P", {"x": "Foo"})
            "TypedDict" => match args.get(1) {
                Some(Expr::Dict(dict)) => dict.items.iter().map(|item| &item.value).collect(),
                _ => Vec::new(),
            },
            // NamedTuple("P", [("x", "Foo")])
            "NamedTuple" => match args.get(1) {
                Some(
                    Expr::List(ast::ExprList { elts, .. })
                    | Expr::Tuple(ast::ExprTuple { elts, .. }),
                ) => elts
                    .iter()
                    .filter_map(|field| match field {
                        Expr::Tuple(pair) => pair.elts.get(1),
                        _ => None,
                    })
                    .collect(),
                _ => Vec::new(),
            },
            _ => return,
        };
        type_exprs.extend(node.arguments.keywords.iter().filter_map(|kw| {
            let arg = kw.arg.as_ref().map(ruff_python_ast::Identifier::as_str);
            let is_type_kw = match name {
                "TypeVar" => matches!(arg, Some("bound" | "default")),
                "cast" => arg == Some("typ"),
                _ => false,
            };
            is_type_kw.then_some(&kw.value)
        }));
        for expr in type_exprs {
            if matches!(expr, Expr::StringLiteral(_)) {
                self.visit_annotation(expr);
            }
        }
    }

    /// Returns whether subscripting `value` builds a type, as with `Union[...]`,
    /// `typing.Optional[...]` or `list[...]`.
    pub(super) fn is_generic_type(&self, value: &Expr) -> bool {
        const TYPE_MODULES: [&str; 3] = ["typing", "typing_extensions", "collections.abc"];
        match value {
            Expr::Name(name) => {
                let id = name.id.as_str();
                match self.alias_map.get(id) {
                    Some(original) => original
                        .rsplit_once('.')
                        .is_some_and(|(module, _)| TYPE_MODULES.contains(&module)),
                    None => matches!(id, "list" | "dict" | "set" | "frozenset" | "tuple" | "type"),
                }
            }
            // `typing.X`, `t.X` (alias) or `collections.abc.X`.
            Expr::Attribute(attr) => match &*attr.value {
                Expr::Name(base) => {
                    let id = base.id.as_str();
                    let module = self.alias_map.get(id).map_or(id, String::as_str);
                    TYPE_MODULES.contains(&module)
                }
                Expr::Attribute(inner) => {
                    inner.attr.as_str() == "abc"
                        && matches!(&*inner.value, Expr::Name(base) if base.id.as_str() == "collections")
                }
                _ => false,
            },
            _ => false,
        }
    }

    /// Returns the `Name` at the root of a `Name`/`Attribute` chain.
    pub(super) fn root_name(expr: &Expr) -> Option<&str> {
        match expr {
            Expr::Name(name) => Some(name.id.as_str()),
            Expr::Attribute(attr) => Self::root_name(&attr.value),
            _ => None,
        }
    }

    /// Returns the dotted name of a `Name`/`Attribute` chain, e.g. `os.path`.
    pub(super) fn dotted_name(expr: &Expr) -> Option<String> {
        match expr {
            Expr::Name(name) => Some(name.id.to_string()),
            Expr::Attribute(attr) => {
                let mut base = Self::dotted_name(&attr.value)?;
                base.push('.');
                base.push_str(attr.attr.as_str());
                Some(base)
            }
            _ => None,
        }
    }
}
