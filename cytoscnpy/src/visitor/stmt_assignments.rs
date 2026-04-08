use super::{ast, CytoScnPyVisitor, DefinitionInfo, DefinitionType, Expr, SmallVec};

impl CytoScnPyVisitor<'_> {
    pub(super) fn handle_assign_stmt(&mut self, node: &ast::StmtAssign) {
        if node.targets.iter().any(Self::is_all_name_expr) {
            if Self::is_all_present_in_expr(&node.value) {
                self.extend_exports_from_expr(&node.value);
            } else {
                self.replace_exports_from_expr(&node.value);
            }
        }

        if self.in_import_error_block {
            for target in &node.targets {
                if let Expr::Name(name_node) = target {
                    let id = name_node.id.as_str();
                    if id.starts_with("HAS_") || id.starts_with("HAVE_") {
                        self.optional_dependency_flags.insert(id.to_owned());
                        self.add_ref(id);
                        if !self.module_name.is_empty() {
                            let qualified = format!("{}.{}", self.module_name, id);
                            self.add_ref(&qualified);
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
                        self.add_ref(name_node.id.as_str());
                        if !self.module_name.is_empty() {
                            let qualified = format!("{}.{}", self.module_name, name_node.id);
                            self.add_ref(&qualified);
                        }
                    } else {
                        let qualified_name = self.get_qualified_name(&name_node.id);
                        let (line, end_line, col, start_byte, end_byte) =
                            self.get_range_info(name_node);
                        self.add_definition(DefinitionInfo {
                            name: qualified_name.clone(),
                            def_type: DefinitionType::Variable,
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
                                self.add_ref(&qualified_name);
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
                            self.add_ref(&qualified_name);
                        }
                    }
                }
            }
        }
    }

    pub(super) fn handle_aug_assign_stmt(&mut self, node: &ast::StmtAugAssign) {
        if Self::is_all_name_expr(&node.target) {
            self.extend_exports_from_expr(&node.value);
        }
        self.visit_expr(&node.target);
        self.visit_expr(&node.value);
    }

    fn is_all_name_expr(expr: &Expr) -> bool {
        matches!(expr, Expr::Name(name) if name.id.as_str() == "__all__")
    }

    fn is_all_present_in_expr(expr: &Expr) -> bool {
        match expr {
            Expr::Name(name) => name.id.as_str() == "__all__",
            Expr::List(list) => list.elts.iter().any(Self::is_all_present_in_expr),
            Expr::Tuple(tuple) => tuple.elts.iter().any(Self::is_all_present_in_expr),
            Expr::BinOp(bin_op) => {
                Self::is_all_present_in_expr(&bin_op.left)
                    || Self::is_all_present_in_expr(&bin_op.right)
            }
            _ => false,
        }
    }

    fn replace_exports_from_expr(&mut self, expr: &Expr) {
        self.exports.clear();
        Self::collect_all_exports(expr, &mut self.exports);
    }

    fn extend_exports_from_expr(&mut self, expr: &Expr) {
        Self::collect_all_exports(expr, &mut self.exports);
    }

    fn collect_all_exports(expr: &Expr, out: &mut Vec<String>) {
        match expr {
            Expr::List(list) => {
                for elt in &list.elts {
                    Self::collect_all_exports(elt, out);
                }
            }
            Expr::Tuple(tuple) => {
                for elt in &tuple.elts {
                    Self::collect_all_exports(elt, out);
                }
            }
            Expr::StringLiteral(string_lit) => out.push(string_lit.value.to_string()),
            Expr::BinOp(bin_op) if bin_op.op == ast::Operator::Add => {
                Self::collect_all_exports(&bin_op.left, out);
                Self::collect_all_exports(&bin_op.right, out);
            }
            _ => {}
        }
    }

    pub(super) fn handle_ann_assign_stmt(&mut self, node: &ast::StmtAnnAssign) {
        if let Expr::Name(name_node) = &*node.target {
            let qualified_name = self.get_qualified_name(&name_node.id);
            let (line, end_line, col, start_byte, end_byte) = self.get_range_info(name_node);
            self.add_definition(DefinitionInfo {
                name: qualified_name.clone(),
                def_type: DefinitionType::Variable,
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
                self.add_ref(&qualified_name);
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
                self.add_ref(&qualified_name);
            }
        }
    }

    pub(super) fn handle_type_alias_stmt(&mut self, node: &ast::StmtTypeAlias) {
        if let Expr::Name(name_node) = &*node.name {
            let qualified_name = self.get_qualified_name(&name_node.id);
            let (line, _, col, start_byte, _) = self.get_range_info(name_node);
            let (_, end_line, _, full_start_byte, end_byte) = self.get_range_info(node);
            self.add_definition(DefinitionInfo {
                name: qualified_name.clone(),
                def_type: DefinitionType::Variable,
                line,
                end_line,
                col,
                start_byte,
                end_byte,
                full_start_byte,
                base_classes: SmallVec::new(),
            });
            self.add_local_def(name_node.id.to_string(), qualified_name);
        } else {
            self.visit_expr(&node.name);
        }

        self.visit_expr(&node.value);
    }
}
