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

            if let Some(module) = &node.module {
                let full_name = format!("{}.{}", module, alias.name);
                self.add_ref(full_name.clone());
                self.alias_map.insert(asname.to_string(), full_name);
            } else {
                self.alias_map
                    .insert(asname.to_string(), alias.name.to_string());
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
}
