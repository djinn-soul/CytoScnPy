use super::*;

fn has_case_insensitive_suffix(value: &str, suffix: &str) -> bool {
    value
        .get(value.len().saturating_sub(suffix.len())..)
        .is_some_and(|tail| tail.eq_ignore_ascii_case(suffix))
}

fn normalize_base_class_name(base: &Expr) -> Option<String> {
    match base {
        Expr::Name(base_name) => Some(base_name.id.to_string()),
        Expr::Attribute(attr) => Some(attr.attr.to_string()),
        // Handles generic bases such as `Protocol[T]` and `typing.Protocol[T]`.
        Expr::Subscript(subscript) => normalize_base_class_name(&subscript.value),
        _ => None,
    }
}

impl<'a> CytoScnPyVisitor<'a> {
    pub(super) fn handle_function_stmt(&mut self, node: &ast::StmtFunctionDef) {
        for decorator in &node.decorator_list {
            self.visit_expr(&decorator.expression);
        }
        self.visit_arguments(&node.parameters);
        if let Some(returns) = &node.returns {
            self.visit_expr(returns);
        }
        self.visit_function_def(
            &node.name,
            &node.decorator_list,
            &node.parameters,
            &node.body,
            node.range(),
        );
    }

    pub(super) fn handle_class_stmt(&mut self, node: &ast::StmtClassDef) {
        let mut is_model_class = false;
        for decorator in &node.decorator_list {
            self.visit_expr(&decorator.expression);
            if let Expr::Name(name) = &decorator.expression {
                if name.id.as_str() == "dataclass" {
                    is_model_class = true;
                }
            } else if let Expr::Call(call) = &decorator.expression {
                if let Expr::Name(func_name) = &*call.func {
                    if func_name.id.as_str() == "dataclass" {
                        is_model_class = true;
                    }
                } else if let Expr::Attribute(attr) = &*call.func {
                    if attr.attr.as_str() == "dataclass" {
                        is_model_class = true;
                    }
                }
            } else if let Expr::Attribute(attr) = &decorator.expression {
                if attr.attr.as_str() == "dataclass" || attr.attr.as_str() == "s" {
                    is_model_class = true;
                }
            }
        }

        let name = &node.name;
        let qualified_name = self.get_qualified_name(name.as_str());
        let name_line = self.line_index.line_index(name.range().start());
        let name_col = self.line_index.column_index(name.range().start());
        let (_, end_line, _, start_byte, end_byte) = self.get_range_info(node);

        let mut base_classes: SmallVec<[String; 2]> = SmallVec::new();
        for base in node.bases() {
            if let Some(base_name) = normalize_base_class_name(base) {
                if matches!(
                    base_name.as_str(),
                    "BaseModel" | "TypedDict" | "NamedTuple" | "Protocol" | "Struct"
                ) {
                    is_model_class = true;
                }
                base_classes.push(base_name);
            }
        }

        let is_protocol = base_classes
            .iter()
            .any(|b| b == "Protocol" || b.ends_with(".Protocol"));
        self.protocol_class_stack.push(is_protocol);

        let is_abc = base_classes
            .iter()
            .any(|b| b == "ABC" || b == "abc.ABC" || has_case_insensitive_suffix(b, ".ABC"));
        self.abc_class_stack.push(is_abc);

        let is_enum = base_classes.iter().any(|b| {
            matches!(
                b.as_str(),
                "Enum" | "IntEnum" | "StrEnum" | "enum.Enum" | "enum.IntEnum" | "enum.StrEnum"
            )
        });
        self.enum_class_stack.push(is_enum);

        self.add_definition(DefinitionInfo {
            name: qualified_name.clone(),
            def_type: DefinitionType::Class,
            line: name_line,
            end_line,
            col: name_col,
            start_byte: node.name.range.start().to_usize(),
            end_byte,
            full_start_byte: start_byte,
            base_classes: base_classes.clone(),
        });
        self.add_local_def(name.to_string(), qualified_name.clone());

        for base in node.bases() {
            self.visit_expr(base);
            if let Expr::Name(base_name) = base {
                self.add_ref(base_name.id.as_str());
                if !self.module_name.is_empty() {
                    let qualified_base = format!("{}.{}", self.module_name, base_name.id);
                    self.add_ref(&qualified_base);
                }
            }
        }

        let mut has_metaclass = false;
        for keyword in node.keywords() {
            self.visit_expr(&keyword.value);
            if keyword
                .arg
                .as_ref()
                .map(ruff_python_ast::Identifier::as_str)
                == Some("metaclass")
            {
                has_metaclass = true;
            }
            if let Expr::Name(kw_name) = &keyword.value {
                self.add_ref(kw_name.id.as_str());
                if !self.module_name.is_empty() {
                    let qualified_kw = format!("{}.{}", self.module_name, kw_name.id);
                    self.add_ref(&qualified_kw);
                }
            }
        }

        if has_metaclass {
            self.metaclass_classes.insert(name.to_string());
            self.metaclass_classes.insert(qualified_name.clone());
        }

        for base_class in &base_classes {
            if self.metaclass_classes.contains(base_class) {
                self.add_ref(&qualified_name);
                self.add_ref(name.as_str());
                break;
            }
        }

        self.class_stack.push(name.to_string());
        self.model_class_stack.push(is_model_class);
        self.enter_scope(ScopeType::Class(CompactString::from(name.as_str())));

        for stmt in &node.body {
            self.visit_stmt(stmt);
        }

        self.class_stack.pop();
        self.model_class_stack.pop();
        self.protocol_class_stack.pop();
        self.abc_class_stack.pop();
        self.enum_class_stack.pop();
        self.exit_scope();
    }
}
