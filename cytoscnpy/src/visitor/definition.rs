use super::{
    Arc, CytoScnPyVisitor, Definition, DefinitionInfo, DefinitionType, Ranged, ScopeType,
    UnusedCategory,
};

struct DefinitionFlags {
    references: usize,
    is_exported: bool,
    in_init: bool,
    is_framework_managed: bool,
    is_enum_member: bool,
    is_constant: bool,
    is_potential_secret: bool,
}

impl CytoScnPyVisitor<'_> {
    pub(super) fn get_range_info<T: Ranged>(
        &self,
        node: &T,
    ) -> (usize, usize, usize, usize, usize) {
        let range = node.range();
        let start_byte = range.start().to_usize();
        let end_byte = range.end().to_usize();
        // line_index uses 1-based indexing for lines
        let start_line = self.line_index.line_index(range.start());
        let end_line = self.line_index.line_index(range.end());
        let col = self.line_index.column_index(range.start());
        (start_line, end_line, col, start_byte, end_byte)
    }

    /// Helper to add a definition using the info struct.
    pub(super) fn add_definition(&mut self, info: DefinitionInfo) {
        let simple_name = info
            .name
            .split('.')
            .next_back()
            .unwrap_or(&info.name)
            .to_owned();
        let flags = self.compute_definition_flags(&info, &simple_name);
        let message = Self::build_definition_message(info.def_type, &simple_name);
        let fix = Self::build_fix_suggestion(&info);

        let definition = Definition {
            name: info.name.clone(),
            full_name: info.name,
            simple_name,
            def_type: info.def_type,
            file: Arc::clone(&self.file_path), // O(1) Arc clone instead of O(n) PathBuf clone
            line: info.line,
            end_line: info.end_line,
            col: info.col,
            start_byte: info.start_byte,
            end_byte: info.end_byte,
            confidence: 100,
            category: UnusedCategory::default(),
            references: flags.references,
            is_exported: flags.is_exported,
            in_init: flags.in_init,
            is_framework_managed: flags.is_framework_managed,
            base_classes: info.base_classes,
            is_type_checking: self.in_type_checking_block,
            is_captured: false,
            cell_number: None,
            is_enum_member: flags.is_enum_member,

            is_self_referential: false,
            message: Some(message),
            fix,
            is_constant: flags.is_constant,
            is_potential_secret: flags.is_potential_secret,
            is_unreachable: false,
        };

        self.definitions.push(definition);
    }

    fn compute_definition_flags(
        &mut self,
        info: &DefinitionInfo,
        simple_name: &str,
    ) -> DefinitionFlags {
        let in_init = self.file_path.ends_with("__init__.py");
        let file_is_test = crate::utils::is_test_path(&self.file_path.to_string_lossy());
        let is_test_function = simple_name.starts_with("test_");
        let is_test_class =
            file_is_test && (simple_name.contains("Test") || simple_name.contains("Suite"));
        let is_test = is_test_function || is_test_class;

        let is_dynamic_pattern = simple_name.starts_with("visit_")
            || simple_name.starts_with("leave_")
            || simple_name.starts_with("on_");
        let is_standard_entry = matches!(simple_name, "main" | "run" | "execute");
        let is_dunder = simple_name.starts_with("__") && simple_name.ends_with("__");

        let is_potential_secret = simple_name.contains("KEY")
            || simple_name.contains("SECRET")
            || simple_name.contains("PASS")
            || simple_name.contains("TOKEN");
        let is_public_api = matches!(self.project_type, crate::config::ProjectType::Library)
            && !simple_name.starts_with('_')
            && info.def_type != DefinitionType::Method
            && !is_potential_secret;

        let is_enum_member = self.enum_class_stack.last().copied().unwrap_or(false);
        let is_class_scope = self
            .scope_stack
            .last()
            .is_some_and(|scope| matches!(scope.kind, ScopeType::Class(_)));
        let is_public_class_attr = is_class_scope
            && info.def_type == DefinitionType::Variable
            && !simple_name.starts_with('_')
            && !is_enum_member;

        let is_constant = self.scope_stack.len() == 1
            && info.def_type == DefinitionType::Variable
            && !simple_name.starts_with('_')
            && !is_potential_secret
            && simple_name.chars().all(|c| !c.is_lowercase())
            && simple_name.chars().any(char::is_uppercase);

        let is_implicitly_used = is_test
            || is_dynamic_pattern
            || is_standard_entry
            || is_dunder
            || is_public_class_attr
            || self.auto_called.contains(simple_name);

        if is_implicitly_used {
            self.add_ref(&info.name);
        }

        DefinitionFlags {
            references: usize::from(is_implicitly_used),
            is_exported: is_implicitly_used || is_public_api,
            in_init,
            is_framework_managed: self
                .scope_stack
                .last()
                .is_some_and(|scope| scope.is_framework),
            is_enum_member,
            is_constant,
            is_potential_secret,
        }
    }

    fn build_definition_message(def_type: DefinitionType, simple_name: &str) -> String {
        match def_type {
            DefinitionType::Method => format!("Method '{simple_name}' is defined but never used"),
            DefinitionType::Class => format!("Class '{simple_name}' is defined but never used"),
            DefinitionType::Import => format!("'{simple_name}' is imported but never used"),
            DefinitionType::Variable => {
                format!("Variable '{simple_name}' is assigned but never used")
            }
            DefinitionType::Parameter => format!("Parameter '{simple_name}' is never used"),
            DefinitionType::Function => format!("'{simple_name}' is defined but never used"),
        }
    }

    fn build_fix_suggestion(
        info: &DefinitionInfo,
    ) -> Option<Box<crate::analyzer::types::FixSuggestion>> {
        (info.full_start_byte < info.end_byte).then(|| {
            Box::new(crate::analyzer::types::FixSuggestion::deletion(
                info.full_start_byte,
                info.end_byte,
            ))
        })
    }
}
