use super::{ast, CytoScnPyVisitor, DefinitionInfo, DefinitionType, SmallVec};

impl CytoScnPyVisitor<'_> {
    pub(super) fn handle_import_stmt(&mut self, node: &ast::StmtImport) {
        let stmt_line = self.get_range_info(node).0;
        for alias in &node.names {
            let simple_name = alias.asname.as_ref().unwrap_or(&alias.name);
            let (line, end_line, col, start_byte, end_byte) = self.get_range_info(alias);
            let qualified_name = self.get_qualified_name(simple_name.as_str());
            self.record_import_alias(alias, line, stmt_line, &qualified_name);

            self.add_definition(DefinitionInfo {
                name: qualified_name.clone(),
                def_type: DefinitionType::Import,
                line,
                end_line,
                col,
                start_byte,
                end_byte,
                full_start_byte: start_byte,
                base_classes: SmallVec::new(),
            });
            // `import a.b` binds `a` in the current scope.
            if alias.asname.is_none() {
                if let Some((head, _)) = alias.name.split_once('.') {
                    self.submodule_imports
                        .entry(head.to_owned())
                        .or_default()
                        .push(qualified_name.clone());
                    let already_bound = self
                        .scope_stack
                        .last()
                        .is_some_and(|scope| scope.local_var_map.contains_key(head));
                    if !already_bound {
                        self.add_local_def(head.to_owned(), qualified_name.clone());
                    }
                }
            }
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
                self.add_ref(&qualified_name);
                self.add_ref(simple_name.as_str());
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

        // `from x import *` — record source module for aggregation-layer resolution;
        // skip normal definition/binding creation since names are not known until
        // the source module's `__all__` is available cross-file.
        if node.names.len() == 1 && node.names[0].name.as_str() == "*" {
            if let Some(base) = &resolved_base_module {
                self.star_imports.push(base.clone());
            }
            return;
        }

        let stmt_line = self.get_range_info(node).0;
        for alias in &node.names {
            let asname = alias.asname.as_ref().unwrap_or(&alias.name);
            let (line, end_line, col, start_byte, end_byte) = self.get_range_info(alias);
            let qualified_name = self.get_qualified_name(asname.as_str());
            self.record_import_alias(alias, line, stmt_line, &qualified_name);

            self.add_definition(DefinitionInfo {
                name: qualified_name.clone(),
                def_type: DefinitionType::Import,
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
            } else {
                self.alias_map
                    .insert(asname.to_string(), alias.name.to_string());
                self.import_bindings.insert(
                    self.get_qualified_name(asname.as_str()),
                    alias.name.to_string(),
                );
                self.add_ref(alias.name.as_str());
            }

            if self.in_import_error_block {
                let qualified_name = if self.module_name.is_empty() {
                    asname.to_string()
                } else {
                    format!("{}.{}", self.module_name, asname)
                };
                self.add_ref(&qualified_name);
                self.add_ref(asname.as_str());
            }
        }
    }

    /// Records where an import alias's statement starts and treats the
    /// redundant alias `import x as x` as an explicit re-export (PEP 484).
    fn record_import_alias(
        &mut self,
        alias: &ast::Alias,
        line: usize,
        stmt_line: usize,
        qualified_name: &str,
    ) {
        if line != stmt_line {
            self.import_statement_lines.insert(line, stmt_line);
        }
        if alias
            .asname
            .as_ref()
            .is_some_and(|asname| asname.as_str() == alias.name.as_str())
        {
            self.add_ref(qualified_name);
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
