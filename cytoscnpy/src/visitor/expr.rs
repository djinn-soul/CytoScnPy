use super::{ast, CytoScnPyVisitor, Expr, Regex, EVAL_IDENTIFIER_RE, MAX_RECURSION_DEPTH};

const RUNTIME_PROTOCOL_REF_PREFIX: &str = "__csp_runtime_protocol__.";

impl CytoScnPyVisitor<'_> {
    pub(super) fn visit_name_expr(&mut self, node: &ast::ExprName) {
        if node.ctx.is_load() {
            let name = node.id.to_string();

            if let Some((qualified, scope_idx)) = self.resolve_name_with_info(&name) {
                if scope_idx < self.scope_stack.len() - 1 {
                    self.captured_definitions.insert(qualified.clone());
                }
                self.add_ref(&qualified);
            } else if self.module_name.is_empty() {
                self.add_ref(&name);
            } else {
                let qualified = format!("{}.{}", self.module_name, name);
                self.add_ref(&qualified);
            }

            if let Some(original) = self.alias_map.get(&name).cloned() {
                if let Some(simple) = original.split('.').next_back() {
                    if simple != original {
                        self.add_ref(simple);
                    }
                }
                self.add_ref(&original);
            }
        }
    }

    pub(super) fn visit_call_expr(&mut self, node: &ast::ExprCall) {
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
        let s = node.value.to_string();
        if !s.contains(' ') && !s.is_empty() {
            self.add_ref(&s);
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
