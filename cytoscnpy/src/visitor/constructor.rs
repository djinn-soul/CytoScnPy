#![allow(missing_docs)]

use super::*;

impl<'a> CytoScnPyVisitor<'a> {
    pub fn new(file_path: PathBuf, module_name: String, line_index: &'a LineIndex) -> Self {
        Self::with_project_type(
            file_path,
            module_name,
            line_index,
            crate::config::ProjectType::default(),
        )
    }

    /// Creates a visitor with an explicit project type for export/public API heuristics.
    pub fn with_project_type(
        file_path: PathBuf,
        module_name: String,
        line_index: &'a LineIndex,
        project_type: crate::config::ProjectType,
    ) -> Self {
        let file_path = Arc::new(file_path); // Wrap in Arc once, share everywhere
        Self {
            definitions: Vec::new(),
            references: FxHashMap::default(),
            exports: Vec::new(),
            dynamic_imports: Vec::new(),
            project_type,
            file_path,
            module_name,
            current_scope: SmallVec::new(),
            class_stack: SmallVec::new(),
            line_index,
            alias_map: FxHashMap::default(),
            import_bindings: FxHashMap::default(),
            function_stack: SmallVec::new(),
            function_params: FxHashMap::default(),
            model_class_stack: SmallVec::new(),
            in_type_checking_block: false,
            scope_stack: smallvec::smallvec![Scope::new(ScopeType::Module)],
            dynamic_scopes: FxHashSet::default(),
            captured_definitions: FxHashSet::default(),
            metaclass_classes: FxHashSet::default(),
            self_referential_methods: FxHashSet::default(),
            cached_scope_prefix: String::new(),
            depth: 0,
            recursion_limit_hit: false,
            auto_called: PYTEST_HOOKS().clone(),
            protocol_class_stack: SmallVec::new(),
            enum_class_stack: SmallVec::new(),
            in_import_error_block: false,
            abc_class_stack: SmallVec::new(),
            abc_abstract_methods: FxHashMap::default(),
            protocol_methods: FxHashMap::default(),
            optional_dependency_flags: FxHashSet::default(),
        }
    }
}
