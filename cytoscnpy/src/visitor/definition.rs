use super::*;

impl<'a> CytoScnPyVisitor<'a> {
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
    #[allow(clippy::too_many_lines)]
    pub(super) fn add_definition(&mut self, info: DefinitionInfo) {
        let simple_name = info
            .name
            .split('.')
            .next_back()
            .unwrap_or(&info.name)
            .to_owned();
        let in_init = self.file_path.ends_with("__init__.py");

        // GENERIC HEURISTICS (No hardcoded project names)

        // 1. Tests: Functions starting with 'test_' are assumed to be Pytest/Unittest tests.
        // These are run by test runners, not called explicitly.
        // 1. Tests: Vulture-style Smart Heuristic
        // If the file looks like a test (tests/ or test_*.py), we are lenient.
        let file_is_test = crate::utils::is_test_path(&self.file_path.to_string_lossy());
        let is_test_function = simple_name.starts_with("test_");

        let is_test_class =
            file_is_test && (simple_name.contains("Test") || simple_name.contains("Suite"));

        let is_test = is_test_function || is_test_class;

        // 2. Dynamic Dispatch Patterns:
        //    - 'visit_' / 'leave_': Standard Visitor pattern (AST, LibCST)
        //    - 'on_': Standard Event Handler pattern (UI libs, callbacks)
        let is_dynamic_pattern = simple_name.starts_with("visit_")
            || simple_name.starts_with("leave_")
            || simple_name.starts_with("on_");

        // 3. Standard Entry Points: Common names for script execution.
        let is_standard_entry = matches!(simple_name.as_str(), "main" | "run" | "execute");

        // Check for module-level constants (UPPER_CASE)
        // These are often configuration or exported constants.
        // BUT exclude potential secrets/keys which should be detected if unused.
        let is_potential_secret = simple_name.contains("KEY")
            || simple_name.contains("SECRET")
            || simple_name.contains("PASS")
            || simple_name.contains("TOKEN");

        // 5. Public API: Symbols not starting with '_' are considered exported/public API.
        //    This is crucial for library analysis where entry points aren't explicit.
        //    FIX: Secrets are NOT public API - they should be flagged if unused.
        let is_public_api = matches!(self.project_type, crate::config::ProjectType::Library)
            && !simple_name.starts_with('_')
            && info.def_type != "method"
            && !is_potential_secret;

        // 4. Dunder Methods: Python's magic methods (__str__, __init__, etc.) are implicitly used.
        let is_dunder = simple_name.starts_with("__") && simple_name.ends_with("__");

        // Check if this is a public class attribute (e.g., `class MyClass: my_attr = 1`)
        let is_class_scope = self
            .scope_stack
            .last()
            .is_some_and(|s| matches!(s.kind, ScopeType::Class(_)));

        // Strict Enum Check: Enum members are NOT implicitly used. They must be referenced.
        let is_enum_member = self.enum_class_stack.last().copied().unwrap_or(false);

        let is_public_class_attr = is_class_scope
            && info.def_type == "variable"
            && !simple_name.starts_with('_')
            && !is_enum_member;

        let is_constant = self.scope_stack.len() == 1
            && info.def_type == "variable"
            && !simple_name.starts_with('_')
            && !is_potential_secret
            && simple_name.chars().all(|c| !c.is_lowercase())
            && simple_name.chars().any(char::is_uppercase);

        // Decision: Is this implicitly used? (For reference counting/suppression)
        let is_implicitly_used = is_test
            || is_dynamic_pattern
            || is_standard_entry
            || is_dunder
            || is_public_class_attr
            || self.auto_called.contains(simple_name.as_str());

        // FIX: Global constants (UPPER_CASE) are NOT "implicitly used" (which would hide them forever).
        // Instead, we let them fall through as unused, BUT we will assign them very low confidence later.
        // This allows --confidence 0 to find unused settings, while keeping default runs clean.

        // Decision: Is this exported? (For Semantic Graph roots)
        let is_exported = is_implicitly_used || is_public_api;

        // Set reference count to 1 if implicitly used to prevent false positives.
        // This treats the definition as "used".
        let references = usize::from(is_implicitly_used);

        // FIX: Ensure the references map is updated for implicitly used items
        // This prevents single_file.rs from overwriting the references count with 0
        if is_implicitly_used {
            self.add_ref(info.name.clone());
        }

        // Generate human-readable message based on def_type
        let message = match info.def_type.as_str() {
            "method" => format!("Method '{simple_name}' is defined but never used"),
            "class" => format!("Class '{simple_name}' is defined but never used"),
            "import" => format!("'{simple_name}' is imported but never used"),
            "variable" => format!("Variable '{simple_name}' is assigned but never used"),
            "parameter" => format!("Parameter '{simple_name}' is never used"),
            _ => format!("'{simple_name}' is defined but never used"),
        };

        // Try to create a fix suggestion if we have valid CST ranges
        // This ensures the JS extension gets ranges even if CST module didn't run
        let fix = if info.full_start_byte < info.end_byte {
            Some(Box::new(crate::analyzer::types::FixSuggestion::deletion(
                info.full_start_byte,
                info.end_byte,
            )))
        } else {
            None
        };

        let is_enum_member = self.enum_class_stack.last().copied().unwrap_or(false);

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
            references,
            is_exported,
            in_init,
            is_framework_managed: self.scope_stack.last().is_some_and(|s| s.is_framework),
            base_classes: info.base_classes,
            is_type_checking: self.in_type_checking_block,
            is_captured: false,
            cell_number: None,
            is_enum_member,

            is_self_referential: false,
            message: Some(message),
            fix,
            is_constant,
            is_potential_secret,
            is_unreachable: false,
        };

        self.definitions.push(definition);
    }
}
