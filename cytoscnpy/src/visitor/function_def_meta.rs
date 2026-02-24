use super::*;

impl<'a> CytoScnPyVisitor<'a> {
    pub(super) fn register_function_definition(
        &mut self,
        name_node: &ruff_python_ast::Identifier,
        range: ruff_text_size::TextRange,
    ) -> String {
        let name = name_node.id.as_str();
        let qualified_name = self.get_qualified_name(name);
        let def_type = if self.class_stack.is_empty() {
            "function"
        } else {
            "method"
        };

        let def_range = name_node.range;
        let start_byte = def_range.start().into();
        let line = self.line_index.line_index(def_range.start());
        let col = self.line_index.column_index(def_range.start());
        let end_byte = range.end().into();
        let end_line = self.line_index.line_index(range.end());

        self.add_definition(DefinitionInfo {
            name: qualified_name.clone(),
            def_type: def_type.to_owned(),
            line,
            end_line,
            col,
            start_byte,
            end_byte,
            full_start_byte: range.start().into(),
            base_classes: SmallVec::new(),
        });
        qualified_name
    }

    pub(super) fn apply_not_implemented_heuristic(&mut self, body: &[ruff_python_ast::Stmt]) {
        let raises_not_implemented = body.iter().any(|s| {
            if let ruff_python_ast::Stmt::Raise(r) = s {
                if let Some(exc) = &r.exc {
                    match &**exc {
                        ruff_python_ast::Expr::Name(n) => return n.id == "NotImplementedError",
                        ruff_python_ast::Expr::Call(c) => {
                            if let ruff_python_ast::Expr::Name(n) = &*c.func {
                                return n.id == "NotImplementedError";
                            }
                        }
                        _ => {}
                    }
                }
            }
            false
        });

        if raises_not_implemented {
            if let Some(last_def) = self.definitions.last_mut() {
                last_def.confidence = 0;
            }
        }
    }

    pub(super) fn collect_interface_method_metadata(
        &mut self,
        name: &str,
        decorator_list: &[ruff_python_ast::Decorator],
    ) {
        if let Some(class_name) = self.class_stack.last() {
            if let Some(true) = self.abc_class_stack.last() {
                let is_abstract = decorator_list.iter().any(|d| {
                    let expr = match &d.expression {
                        ruff_python_ast::Expr::Call(call) => &*call.func,
                        _ => &d.expression,
                    };
                    match expr {
                        ruff_python_ast::Expr::Name(n) => n.id == "abstractmethod",
                        ruff_python_ast::Expr::Attribute(attr) => {
                            attr.attr.as_str() == "abstractmethod"
                        }
                        _ => false,
                    }
                });

                if is_abstract {
                    self.abc_abstract_methods
                        .entry(class_name.clone())
                        .or_default()
                        .insert(name.to_owned());
                    if let Some(def) = self.definitions.last_mut() {
                        def.confidence = 0;
                    }
                }
            }

            if let Some(true) = self.protocol_class_stack.last() {
                self.protocol_methods
                    .entry(class_name.clone())
                    .or_default()
                    .insert(name.to_owned());
            }
        }
    }

    pub(super) fn mark_framework_function(
        &mut self,
        decorator_list: &[ruff_python_ast::Decorator],
    ) -> bool {
        let mut should_add_ref = false;
        if let Some(scope) = self.scope_stack.last_mut() {
            for decorator in decorator_list {
                let expr = match &decorator.expression {
                    ruff_python_ast::Expr::Call(call) => &*call.func,
                    _ => &decorator.expression,
                };

                if let ruff_python_ast::Expr::Attribute(attr) = expr {
                    if let ruff_python_ast::Expr::Name(name) = &*attr.value {
                        let base = name.id.as_str();
                        if matches!(base, "app" | "router" | "celery") {
                            scope.is_framework = true;
                            if base == "app" {
                                if let Some(def) = self.definitions.last_mut() {
                                    def.is_framework_managed = true;
                                    def.is_exported = true;
                                    if def.references == 0 {
                                        def.references = 1;
                                    }
                                }
                                should_add_ref = true;
                            } else if let Some(def) = self.definitions.last_mut() {
                                def.is_framework_managed = true;
                            }
                        }
                    }
                }
            }
        }
        should_add_ref
    }

    pub(super) fn should_skip_parameters(
        &self,
        decorator_list: &[ruff_python_ast::Decorator],
    ) -> bool {
        if self.protocol_class_stack.last().is_some_and(|v| *v) {
            return true;
        }

        for decorator in decorator_list {
            let expr = match &decorator.expression {
                ruff_python_ast::Expr::Call(call) => &*call.func,
                _ => &decorator.expression,
            };

            if let ruff_python_ast::Expr::Name(name) = expr {
                if name.id == "abstractmethod" || name.id == "overload" {
                    return true;
                }
            } else if let ruff_python_ast::Expr::Attribute(attr) = expr {
                if attr.attr.as_str() == "abstractmethod" || attr.attr.as_str() == "overload" {
                    return true;
                }
            }
        }
        false
    }
}
