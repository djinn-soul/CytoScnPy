use super::*;

impl<'a> CytoScnPyVisitor<'a> {
    pub(super) fn handle_import_stmt(&mut self, node: &ast::StmtImport) {
        for alias in &node.names {
            let simple_name = alias.asname.as_ref().unwrap_or(&alias.name);
            let (line, end_line, col, start_byte, end_byte) = self.get_range_info(alias);
            let qualified_name = self.get_qualified_name(simple_name.as_str());

            self.add_definition(DefinitionInfo {
                name: qualified_name.clone(),
                def_type: "import".to_owned(),
                line,
                end_line,
                col,
                start_byte,
                end_byte,
                full_start_byte: start_byte,
                base_classes: SmallVec::new(),
            });
            self.add_local_def(simple_name.as_str().to_owned(), qualified_name);

            self.alias_map
                .insert(simple_name.to_string(), alias.name.to_string());
            self.import_bindings.insert(
                self.get_qualified_name(simple_name.as_str()),
                alias.name.to_string(),
            );

            if self.in_import_error_block {
                let qualified_name = if self.module_name.is_empty() {
                    simple_name.to_string()
                } else {
                    format!("{}.{}", self.module_name, simple_name)
                };
                self.add_ref(qualified_name);
                self.add_ref(simple_name.to_string());
            }
        }
    }

    pub(super) fn handle_import_from_stmt(&mut self, node: &ast::StmtImportFrom) {
        if let Some(module) = &node.module {
            if module == "__future__" {
                return;
            }
        }

        let resolved_base_module = self.resolve_import_from_base_module(node);

        for alias in &node.names {
            let asname = alias.asname.as_ref().unwrap_or(&alias.name);
            let (line, end_line, col, start_byte, end_byte) = self.get_range_info(alias);
            let qualified_name = self.get_qualified_name(asname.as_str());

            self.add_definition(DefinitionInfo {
                name: qualified_name.clone(),
                def_type: "import".to_owned(),
                line,
                end_line,
                col,
                start_byte,
                end_byte,
                full_start_byte: start_byte,
                base_classes: SmallVec::new(),
            });
            self.add_local_def(asname.to_string(), qualified_name);

            if let Some(base_module) = resolved_base_module.as_deref() {
                let full_name = format!("{base_module}.{}", alias.name);
                self.alias_map.insert(asname.to_string(), full_name.clone());
                self.import_bindings
                    .insert(self.get_qualified_name(asname.as_str()), full_name.clone());
                // Importing a symbol is itself a static dependency on that source symbol.
                self.add_ref(full_name);
            } else {
                self.alias_map
                    .insert(asname.to_string(), alias.name.to_string());
                self.import_bindings.insert(
                    self.get_qualified_name(asname.as_str()),
                    alias.name.to_string(),
                );
                self.add_ref(alias.name.to_string());
            }

            if self.in_import_error_block {
                let qualified_name = if self.module_name.is_empty() {
                    asname.to_string()
                } else {
                    format!("{}.{}", self.module_name, asname)
                };
                self.add_ref(qualified_name);
                self.add_ref(asname.to_string());
            }
        }
    }

    fn resolve_import_from_base_module(&self, node: &ast::StmtImportFrom) -> Option<String> {
        let level = node.level as usize;
        if level == 0 {
            return node.module.as_ref().map(ToString::to_string);
        }

        let mut package_parts: Vec<&str> = self
            .module_name
            .split('.')
            .filter(|part| !part.is_empty())
            .collect();

        let is_init_module = self
            .file_path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .is_some_and(|stem| stem == "__init__");

        if !is_init_module {
            let _ = package_parts.pop();
        }

        for _ in 1..level {
            if package_parts.pop().is_none() {
                break;
            }
        }

        if let Some(module) = &node.module {
            package_parts.push(module.as_str());
        }

        if package_parts.is_empty() {
            node.module.as_ref().map(ToString::to_string)
        } else {
            Some(package_parts.join("."))
        }
    }
}
