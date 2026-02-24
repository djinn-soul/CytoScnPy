#![allow(missing_docs)]

use super::*;

impl<'a> CytoScnPyVisitor<'a> {
    pub(super) fn visit_name_expr(&mut self, node: &ast::ExprName) {
        if node.ctx.is_load() {
            let name = node.id.to_string();

            if let Some((qualified, scope_idx)) = self.resolve_name_with_info(&name) {
                if scope_idx < self.scope_stack.len() - 1 {
                    self.captured_definitions.insert(qualified.clone());
                }
                self.add_ref(qualified);
            } else if self.module_name.is_empty() {
                self.add_ref(name.clone());
            } else {
                self.add_ref(format!("{}.{}", self.module_name, name));
            }

            if let Some(original) = self.alias_map.get(&name).cloned() {
                if let Some(simple) = original.split('.').next_back() {
                    if simple != original {
                        self.add_ref(simple.to_owned());
                    }
                }
                self.add_ref(original);
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
                            self.add_ref(m.as_str().to_owned());
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
                    self.add_ref(attr_ref);
                    if !self.module_name.is_empty() {
                        let full_attr_ref =
                            format!("{}.{}.{}", self.module_name, obj_name.id, attr_value);
                        self.add_ref(full_attr_ref);
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
                    self.add_ref(attr_ref);
                    if !self.module_name.is_empty() {
                        let full_attr_ref =
                            format!("{}.{}.{}", self.module_name, obj_name.id, attr_value);
                        self.add_ref(full_attr_ref);
                    }
                }
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

    pub(super) fn visit_attribute_expr(&mut self, node: &ast::ExprAttribute) {
        self.add_ref(format!(".{}", node.attr));

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
                self.add_ref(original_base.clone());
                let full_attr = format!("{}.{}", original_base, node.attr);
                self.add_ref(full_attr);
            }

            if (base_id == "self" || base_id == "cls") && !self.class_stack.is_empty() {
                let method_name = &node.attr;
                let mut parts = Vec::new();
                if !self.module_name.is_empty() {
                    parts.push(self.module_name.clone());
                }
                parts.extend(self.class_stack.clone());
                parts.push(method_name.to_string());
                self.add_ref(parts.join("."));
            } else {
                self.add_ref(base_id.to_owned());
                let full_attr = format!("{}.{}", base_id, node.attr);
                self.add_ref(full_attr);
            }
        }
        self.visit_expr(&node.value);
    }

    pub(super) fn visit_string_literal(&mut self, node: &ast::ExprStringLiteral) {
        let s = node.value.to_string();
        if !s.contains(' ') && !s.is_empty() {
            self.add_ref(s.clone());
            if !self.module_name.is_empty() {
                self.add_ref(format!("{}.{}", self.module_name, s));
            }

            let mut current_word = String::new();
            for ch in s.chars() {
                if ch.is_alphanumeric() || ch == '_' {
                    current_word.push(ch);
                } else if !current_word.is_empty() {
                    if current_word.chars().next().is_some_and(char::is_uppercase) {
                        self.add_ref(current_word.clone());
                        if !self.module_name.is_empty() {
                            self.add_ref(format!("{}.{}", self.module_name, current_word));
                        }
                    }
                    current_word.clear();
                }
            }
            if !current_word.is_empty()
                && current_word.chars().next().is_some_and(char::is_uppercase)
            {
                self.add_ref(current_word.clone());
                if !self.module_name.is_empty() {
                    self.add_ref(format!("{}.{}", self.module_name, current_word));
                }
            }
        }
    }

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
