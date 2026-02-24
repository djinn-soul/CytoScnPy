use super::*;

impl<'a> CytoScnPyVisitor<'a> {
    pub(super) fn handle_assign_stmt(&mut self, node: &ast::StmtAssign) {
        if let Some(Expr::Name(target)) = node.targets.first() {
            if target.id.as_str() == "__all__" {
                if let Expr::List(list) = &*node.value {
                    for elt in &list.elts {
                        if let Expr::StringLiteral(string_lit) = elt {
                            self.exports.push(string_lit.value.to_string());
                        }
                    }
                }
            }
        }

        if self.in_import_error_block {
            for target in &node.targets {
                if let Expr::Name(name_node) = target {
                    let id = name_node.id.as_str();
                    if id.starts_with("HAS_") || id.starts_with("HAVE_") {
                        self.optional_dependency_flags.insert(id.to_owned());
                        self.add_ref(id.to_owned());
                        if !self.module_name.is_empty() {
                            let qualified = format!("{}.{}", self.module_name, id);
                            self.add_ref(qualified);
                        }
                    }
                }
            }
        }
        self.visit_expr(&node.value);

        for target in &node.targets {
            if let Expr::Name(name_node) = target {
                if name_node.id.as_str() != "__all__" {
                    let is_global = self
                        .scope_stack
                        .last()
                        .is_some_and(|s| s.global_declarations.contains(name_node.id.as_str()));

                    if is_global {
                        self.add_ref(name_node.id.to_string());
                        if !self.module_name.is_empty() {
                            let qualified = format!("{}.{}", self.module_name, name_node.id);
                            self.add_ref(qualified);
                        }
                    } else {
                        let qualified_name = self.get_qualified_name(&name_node.id);
                        let (line, end_line, col, start_byte, end_byte) =
                            self.get_range_info(name_node);
                        self.add_definition(DefinitionInfo {
                            name: qualified_name.clone(),
                            def_type: "variable".to_owned(),
                            line,
                            end_line,
                            col,
                            start_byte,
                            end_byte,
                            full_start_byte: start_byte,
                            base_classes: SmallVec::new(),
                        });
                        self.add_local_def(name_node.id.to_string(), qualified_name.clone());

                        if !self.class_stack.is_empty() && self.function_stack.is_empty() {
                            if let Some(true) = self.model_class_stack.last() {
                                self.add_ref(qualified_name);
                            }
                        }
                    }
                }
            } else {
                self.visit_expr(target);
            }
        }

        if let Expr::Call(call) = &*node.value {
            if let Expr::Name(func_name) = &*call.func {
                let fname = func_name.id.as_str();
                if fname == "TypeAliasType" || fname == "NewType" {
                    for target in &node.targets {
                        if let Expr::Name(name_node) = target {
                            let qualified_name = self.get_qualified_name(&name_node.id);
                            self.add_ref(qualified_name);
                        }
                    }
                }
            }
        }
    }

    pub(super) fn handle_ann_assign_stmt(&mut self, node: &ast::StmtAnnAssign) {
        if let Expr::Name(name_node) = &*node.target {
            let qualified_name = self.get_qualified_name(&name_node.id);
            let (line, end_line, col, start_byte, end_byte) = self.get_range_info(name_node);
            self.add_definition(DefinitionInfo {
                name: qualified_name.clone(),
                def_type: "variable".to_owned(),
                line,
                end_line,
                col,
                start_byte,
                end_byte,
                full_start_byte: start_byte,
                base_classes: SmallVec::new(),
            });
            self.add_local_def(name_node.id.to_string(), qualified_name.clone());

            if !self.class_stack.is_empty()
                && self.function_stack.is_empty()
                && self.model_class_stack.last().is_some_and(|v| *v)
            {
                self.add_ref(qualified_name);
            }
        } else {
            self.visit_expr(&node.target);
        }

        self.visit_expr(&node.annotation);
        if let Some(value) = &node.value {
            self.visit_expr(value);
        }

        let mut is_type_alias = false;
        if let Expr::Name(ann_name) = &*node.annotation {
            if ann_name.id == "TypeAlias" {
                is_type_alias = true;
            }
        } else if let Expr::Attribute(ann_attr) = &*node.annotation {
            if ann_attr.attr.as_str() == "TypeAlias" {
                is_type_alias = true;
            }
        }

        if is_type_alias {
            if let Expr::Name(name_node) = &*node.target {
                let qualified_name = self.get_qualified_name(&name_node.id);
                self.add_ref(qualified_name);
            }
        }
    }
}
