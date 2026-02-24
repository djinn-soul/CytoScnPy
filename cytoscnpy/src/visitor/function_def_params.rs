use super::*;

impl<'a> CytoScnPyVisitor<'a> {
    pub(super) fn collect_function_parameters(
        &mut self,
        parameters: &ruff_python_ast::Parameters,
        qualified_name: &str,
        skip_parameters: bool,
    ) -> FxHashSet<String> {
        let mut param_names = FxHashSet::default();

        for arg in &parameters.posonlyargs {
            let param_name = arg.parameter.name.to_string();
            param_names.insert(param_name.clone());
            self.register_regular_param(qualified_name, param_name, arg, skip_parameters);
        }

        for arg in &parameters.args {
            let param_name = arg.parameter.name.to_string();
            param_names.insert(param_name.clone());
            self.register_regular_param(qualified_name, param_name, arg, skip_parameters);
        }

        for arg in &parameters.kwonlyargs {
            let param_name = arg.parameter.name.to_string();
            param_names.insert(param_name.clone());
            let param_qualified = format!("{qualified_name}.{param_name}");
            self.add_local_def(param_name, param_qualified.clone());
            if !skip_parameters {
                self.add_parameter_definition(param_qualified, arg);
            }
        }

        if let Some(vararg) = &parameters.vararg {
            let param_name = vararg.name.to_string();
            param_names.insert(param_name.clone());
            let param_qualified = format!("{qualified_name}.{param_name}");
            self.add_local_def(param_name, param_qualified.clone());
            if !skip_parameters {
                self.add_parameter_definition(param_qualified, &**vararg);
            }
        }

        if let Some(kwarg) = &parameters.kwarg {
            let param_name = kwarg.name.to_string();
            param_names.insert(param_name.clone());
            let param_qualified = format!("{qualified_name}.{param_name}");
            self.add_local_def(param_name, param_qualified.clone());
            if !skip_parameters {
                self.add_parameter_definition(param_qualified, &**kwarg);
            }
        }

        param_names
    }

    pub(super) fn register_regular_param<T: Ranged>(
        &mut self,
        qualified_name: &str,
        param_name: String,
        node: &T,
        skip_parameters: bool,
    ) {
        let param_qualified = if param_name != "self" && param_name != "cls" {
            format!("{qualified_name}.{param_name}")
        } else {
            param_name.clone()
        };
        self.add_local_def(param_name.clone(), param_qualified.clone());
        if !skip_parameters && param_name != "self" && param_name != "cls" {
            self.add_parameter_definition(param_qualified, node);
        }
    }

    pub(super) fn add_parameter_definition<T: Ranged>(
        &mut self,
        param_qualified: String,
        node: &T,
    ) {
        let (line, end_line, col, start_byte, end_byte) = self.get_range_info(node);
        self.add_definition(DefinitionInfo {
            name: param_qualified,
            def_type: "parameter".to_owned(),
            line,
            end_line,
            col,
            start_byte,
            end_byte,
            full_start_byte: start_byte,
            base_classes: SmallVec::new(),
        });
    }
}
