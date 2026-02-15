use super::*;

impl<'a> CytoScnPyVisitor<'a> {
    pub(super) fn visit_function_def(
        &mut self,
        name_node: &ruff_python_ast::Identifier,
        decorator_list: &[ruff_python_ast::Decorator],
        parameters: &ruff_python_ast::Parameters,
        body: &[ruff_python_ast::Stmt],
        range: ruff_text_size::TextRange,
    ) {
        let name = name_node.id.as_str();
        let qualified_name = self.register_function_definition(name_node, range);

        self.apply_not_implemented_heuristic(body);
        self.collect_interface_method_metadata(name, decorator_list);
        self.add_local_def(name.to_owned(), qualified_name.clone());
        self.enter_scope(ScopeType::Function(CompactString::from(name)));

        if self.mark_framework_function(decorator_list) {
            self.add_ref(qualified_name.clone());
        }

        let skip_parameters = self.should_skip_parameters(decorator_list);
        let param_names =
            self.collect_function_parameters(parameters, &qualified_name, skip_parameters);
        self.function_params
            .insert(qualified_name.clone(), param_names);

        self.function_stack.push(qualified_name);
        for stmt in body {
            self.visit_stmt(stmt);
        }
        self.function_stack.pop();
        self.exit_scope();
    }
}
