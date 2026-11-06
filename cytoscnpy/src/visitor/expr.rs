use super::scope_ops::bump_ref;
use super::{ast, CytoScnPyVisitor, Expr, Regex, EVAL_IDENTIFIER_RE, MAX_RECURSION_DEPTH};

const RUNTIME_PROTOCOL_REF_PREFIX: &str = "__csp_runtime_protocol__.";

impl CytoScnPyVisitor<'_> {
    pub(super) fn visit_name_expr(&mut self, node: &ast::ExprName) {
        if node.ctx.is_load() {
            let name = node.id.as_str();

            if let Some((qualified, scope_idx)) = self.resolve_name_with_info(name) {
                if scope_idx < self.scope_stack.len() - 1 {
                    self.captured_definitions.insert(qualified.clone());
                }
                self.add_ref(&qualified);
            } else if self.module_name.is_empty() {
                self.add_ref(name);
            } else {
                let qualified = format!("{}.{}", self.module_name, name);
                self.add_ref(&qualified);
            }

            if let Some(original) = self.alias_map.get(name).cloned() {
                if let Some(simple) = original.split('.').next_back() {
                    if simple != original {
                        self.add_ref(simple);
                    }
                }
                self.add_ref(&original);
            }

            if let Some(submodules) = self.submodule_imports.get(name) {
                for qualified in submodules {
                    bump_ref(&mut self.references, qualified);
                }
            }
        }
    }

    pub(super) fn visit_call_expr(&mut self, node: &ast::ExprCall) {
        match &*node.func {
            Expr::Name(func_name) => {
                self.visit_typing_call_string_args(func_name.id.as_str(), node);
            }
            Expr::Attribute(attr) => self.visit_typing_call_string_args(attr.attr.as_str(), node),
            _ => {}
        }
        if let Expr::Name(func_name) = &*node.func {
            let name = func_name.id.as_str();
            if name == "eval" {
                let mut handled_as_literal = false;
                if let Some(Expr::StringLiteral(s)) = node.arguments.args.first() {
                    let val = s.value.to_string();
                    if let Some(re) =
                        EVAL_IDENTIFIER_RE.get_or_init(|| Regex::new(r"\b[a-zA-Z_]\w*\b").ok())
                    {
                        for m in re.find_iter(&val) {
                            self.add_ref(m.as_str());
                        }
                    }
                    handled_as_literal = true;
                }

                if !handled_as_literal {
                    let scope_id = self.get_current_scope_id();
                    self.dynamic_scopes.insert(scope_id);
                }
            } else if name == "exec" || name == "globals" || name == "locals" {
                let scope_id = self.get_current_scope_id();
                self.dynamic_scopes.insert(scope_id);
            } else if name == "getattr" && node.arguments.args.len() >= 2 {
                if let (Expr::Name(obj_name), Expr::StringLiteral(attr_str)) =
                    (&node.arguments.args[0], &node.arguments.args[1])
                {
                    let attr_value = attr_str.value.to_string();
                    let attr_ref = format!("{}.{}", obj_name.id, attr_value);
                    self.add_ref(&attr_ref);
                    if !self.module_name.is_empty() {
                        let full_attr_ref =
                            format!("{}.{}.{}", self.module_name, obj_name.id, attr_value);
                        self.add_ref(&full_attr_ref);
                    }
                } else {
                    let scope_id = self.get_current_scope_id();
                    self.dynamic_scopes.insert(scope_id);
                }
            }

            if name == "hasattr" && node.arguments.args.len() == 2 {
                if let (Expr::Name(obj_name), Expr::StringLiteral(attr_str)) =
                    (&node.arguments.args[0], &node.arguments.args[1])
                {
                    let attr_value = attr_str.value.to_string();
                    let attr_ref = format!("{}.{}", obj_name.id, attr_value);
                    self.add_ref(&attr_ref);
                    if !self.module_name.is_empty() {
                        let full_attr_ref =
                            format!("{}.{}.{}", self.module_name, obj_name.id, attr_value);
                        self.add_ref(&full_attr_ref);
                    }
                }
            }

            if (name == "isinstance" || name == "issubclass") && node.arguments.args.len() >= 2 {
                self.record_runtime_protocol_hints(&node.arguments.args[1]);
            }

            if name == "__import__" {
                if let Some(Expr::StringLiteral(module_str)) = node.arguments.args.first() {
                    self.dynamic_imports.push(module_str.value.to_string());
                }
            } else if name == "import_module" {
                let imported_via_alias = self
                    .alias_map
                    .get(name)
                    .is_some_and(|v| v == "importlib.import_module");
                if imported_via_alias {
                    if let Some(Expr::StringLiteral(module_str)) = node.arguments.args.first() {
                        self.dynamic_imports.push(module_str.value.to_string());
                    }
                }
            }
        } else if let Expr::Attribute(attr) = &*node.func {
            if attr.attr.as_str() == "import_module" {
                let is_importlib_call = match &*attr.value {
                    Expr::Name(name) if name.id.as_str() == "importlib" => true,
                    Expr::Name(name) => self
                        .alias_map
                        .get(name.id.as_str())
                        .is_some_and(|v| v == "importlib"),
                    _ => false,
                };

                if is_importlib_call {
                    if let Some(Expr::StringLiteral(module_str)) = node.arguments.args.first() {
                        self.dynamic_imports.push(module_str.value.to_string());
                    }
                }
            }
        }

        let is_pytest_usefixtures_call = Self::is_pytest_usefixtures_call(node);
        self.visit_expr(&node.func);
        for arg in &node.arguments.args {
            if is_pytest_usefixtures_call && matches!(arg, Expr::StringLiteral(_)) {
                continue;
            }
            self.visit_expr(arg);
        }
        for keyword in &node.arguments.keywords {
            self.visit_expr(&keyword.value);
        }
    }

    pub(super) fn is_pytest_usefixtures_call(node: &ast::ExprCall) -> bool {
        match &*node.func {
            Expr::Attribute(attr) if attr.attr.as_str() == "usefixtures" => match &*attr.value {
                Expr::Attribute(inner) => inner.attr.as_str() == "mark",
                Expr::Name(name) => name.id.as_str() == "mark",
                _ => false,
            },
            _ => false,
        }
    }

    fn runtime_protocol_name(expr: &Expr) -> Option<String> {
        match expr {
            Expr::Name(name) => Some(name.id.to_string()),
            Expr::Attribute(attr) => {
                if let Expr::Name(base_name) = &*attr.value {
                    Some(format!("{}.{}", base_name.id, attr.attr))
                } else {
                    Some(attr.attr.to_string())
                }
            }
            Expr::Subscript(subscript) => Self::runtime_protocol_name(&subscript.value),
            _ => None,
        }
    }

    fn record_runtime_protocol_hints(&mut self, type_expr: &Expr) {
        match type_expr {
            Expr::Tuple(tuple_expr) => {
                for element in &tuple_expr.elts {
                    self.record_runtime_protocol_hints(element);
                }
            }
            _ => {
                if let Some(protocol_name) = Self::runtime_protocol_name(type_expr) {
                    self.add_ref(&format!("{RUNTIME_PROTOCOL_REF_PREFIX}{protocol_name}"));
                }
            }
        }
    }

    pub(super) fn visit_attribute_expr(&mut self, node: &ast::ExprAttribute) {
        let attr_name = node.attr.as_str();
        // `import os.path` is bound under the dotted key `os.path`. Building
        // the chain is skipped unless its root has such an import.
        if Self::root_name(&node.value)
            .is_some_and(|root| self.submodule_imports.contains_key(root))
        {
            if let Some(mut dotted) = Self::dotted_name(&node.value) {
                dotted.push('.');
                dotted.push_str(attr_name);
                if let Some((qualified, _)) = self.resolve_name_with_info(&dotted) {
                    self.add_ref(&qualified);
                }
            }
        }
        let mut attr_ref = String::with_capacity(attr_name.len() + 1);
        attr_ref.push('.');
        attr_ref.push_str(attr_name);
        self.add_ref(&attr_ref);

        if let Expr::Name(base_node) = &*node.value {
            if base_node.id.as_str() == "self" || base_node.id.as_str() == "cls" {
                let attr_name = node.attr.as_str();
                if let Some(current_method_qualified) = self.function_stack.last() {
                    let current_method_simple =
                        if let Some(idx) = current_method_qualified.rfind('.') {
                            &current_method_qualified[idx + 1..]
                        } else {
                            current_method_qualified.as_str()
                        };

                    if current_method_simple == attr_name {
                        self.self_referential_methods
                            .insert(current_method_qualified.clone());
                    }
                }
            }
        }

        if let Expr::Name(name_node) = &*node.value {
            let base_id = name_node.id.as_str();
            let original_base_opt = self.alias_map.get(base_id).cloned();
            if let Some(original_base) = original_base_opt {
                self.add_ref(&original_base);
                let mut full_attr =
                    String::with_capacity(original_base.len() + 1 + attr_name.len());
                full_attr.push_str(&original_base);
                full_attr.push('.');
                full_attr.push_str(attr_name);
                self.add_ref(&full_attr);
            }

            if (base_id == "self" || base_id == "cls") && !self.class_stack.is_empty() {
                let mut total_len = attr_name.len();
                if !self.module_name.is_empty() {
                    total_len += self.module_name.len() + 1;
                }
                for class_name in &self.class_stack {
                    total_len += class_name.len() + 1;
                }

                let mut qualified = String::with_capacity(total_len);
                if !self.module_name.is_empty() {
                    qualified.push_str(&self.module_name);
                }
                for class_name in &self.class_stack {
                    if !qualified.is_empty() {
                        qualified.push('.');
                    }
                    qualified.push_str(class_name);
                }
                if !qualified.is_empty() {
                    qualified.push('.');
                }
                qualified.push_str(attr_name);
                self.add_ref(&qualified);
            } else {
                self.add_ref(base_id);
                let mut full_attr = String::with_capacity(base_id.len() + 1 + attr_name.len());
                full_attr.push_str(base_id);
                full_attr.push('.');
                full_attr.push_str(attr_name);
                self.add_ref(&full_attr);
            }
        }
        self.visit_expr(&node.value);
    }

    pub(super) fn visit_string_literal(&mut self, node: &ast::ExprStringLiteral) {
        let s = node.value.to_str();
        if self.in_annotation {
            // Forward reference: `x: "OrderedDict[str, Any] | None"`.
            if let Ok(parsed) = ruff_python_parser::parse_expression(s.trim()) {
                self.visit_expr(parsed.expr());
                return;
            }
        }
        if !s.contains(' ') && !s.is_empty() {
            self.add_ref(s);
            if !self.module_name.is_empty() {
                let qualified = format!("{}.{}", self.module_name, s);
                self.add_ref(&qualified);
            }

            if !s.chars().any(char::is_uppercase) {
                return;
            }

            for token in s.split(|ch: char| !ch.is_alphanumeric() && ch != '_') {
                if token.chars().next().is_some_and(char::is_uppercase) {
                    self.add_ref(token);
                    if !self.module_name.is_empty() {
                        let qualified = format!("{}.{}", self.module_name, token);
                        self.add_ref(&qualified);
                    }
                }
            }
        }
    }

    /// Visits a type annotation. String literals inside it are treated as
    /// forward references and parsed, so names used only there count as used.
    pub(super) fn visit_annotation(&mut self, expr: &Expr) {
        let prev = std::mem::replace(&mut self.in_annotation, true);
        self.visit_expr(expr);
        self.in_annotation = prev;
    }

    /// Visits a non-type expression nested inside an annotation, such as the
    /// values of `Literal[...]` or the metadata of `Annotated[...]`.
    fn visit_non_annotation(&mut self, expr: &Expr) {
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
    fn visit_typing_call_string_args(&mut self, name: &str, node: &ast::ExprCall) {
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
    fn root_name(expr: &Expr) -> Option<&str> {
        match expr {
            Expr::Name(name) => Some(name.id.as_str()),
            Expr::Attribute(attr) => Self::root_name(&attr.value),
            _ => None,
        }
    }

    /// Returns the dotted name of a `Name`/`Attribute` chain, e.g. `os.path`.
    fn dotted_name(expr: &Expr) -> Option<String> {
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

    /// Visits an expression node and recursively traverses its children.
    pub fn visit_expr(&mut self, expr: &Expr) {
        if self.depth >= MAX_RECURSION_DEPTH {
            self.recursion_limit_hit = true;
            return;
        }
        self.depth += 1;
        self.visit_expr_children(expr);
        self.depth -= 1;
    }
}
