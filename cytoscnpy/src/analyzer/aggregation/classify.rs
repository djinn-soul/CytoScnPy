use super::reachability::ReachabilityContext;
use super::usage::{ClassifyContext, Outcome};
use crate::visitor::{Definition, DefinitionType};
use crate::whitelist::WhitelistMatcher;
use rayon::prelude::*;
use rustc_hash::{FxHashMap, FxHashSet};

pub(super) struct ClassificationResult {
    pub(super) unused_functions: Vec<Definition>,
    pub(super) unused_methods: Vec<Definition>,
    pub(super) unused_classes: Vec<Definition>,
    pub(super) unused_imports: Vec<Definition>,
    pub(super) unused_variables: Vec<Definition>,
    pub(super) unused_parameters: Vec<Definition>,
    pub(super) methods_with_refs: Vec<Definition>,
}

impl ClassificationResult {
    fn new() -> Self {
        Self {
            unused_functions: Vec::new(),
            unused_methods: Vec::new(),
            unused_classes: Vec::new(),
            unused_imports: Vec::new(),
            unused_variables: Vec::new(),
            unused_parameters: Vec::new(),
            methods_with_refs: Vec::new(),
        }
    }
}

pub(super) fn classify_definitions(
    definitions: Vec<Definition>,
    ref_counts: &FxHashMap<String, usize>,
    prod_ref_counts: &FxHashMap<String, usize>,
    reachability: &ReachabilityContext,
    fixture_definition_names: &FxHashSet<String>,
    confidence_threshold: u8,
    whitelist: Option<&WhitelistMatcher>,
    analysis_root: &std::path::Path,
) -> ClassificationResult {
    let mut result = ClassificationResult::new();
    // Resolve parent kinds up front so the map can borrow names instead of
    // cloning every one before `definitions` is consumed below.
    let nested_under_callable: Vec<bool> = {
        // Only callable and class parents matter. A later definition of the
        // same name still replaces an earlier one, as in a full name map.
        let mut definition_kinds: FxHashMap<&str, DefinitionType> = FxHashMap::default();
        for def in &definitions {
            if matches!(
                def.def_type,
                DefinitionType::Function | DefinitionType::Method | DefinitionType::Class
            ) {
                definition_kinds.insert(&def.full_name, def.def_type);
            } else {
                definition_kinds.remove(def.full_name.as_str());
            }
        }
        definitions
            .par_iter()
            .map(|def| has_parent_callable(def, &definition_kinds))
            .collect()
    };

    // Definitions arrive grouped by file and share its `Arc`, so check the
    // path once per file.
    let mut last_test_file: Option<(&std::sync::Arc<std::path::PathBuf>, bool)> = None;
    let in_test_file: Vec<bool> = definitions
        .iter()
        .map(|def| match last_test_file {
            Some((file, is_test)) if std::sync::Arc::ptr_eq(file, &def.file) => is_test,
            _ => {
                let def_relative = def
                    .file
                    .strip_prefix(analysis_root)
                    .unwrap_or(def.file.as_path());
                let is_test = crate::utils::is_test_path(&def_relative.to_string_lossy());
                last_test_file = Some((&def.file, is_test));
                is_test
            }
        })
        .collect();

    debug_assert_eq!(reachability.flags.len(), in_test_file.len());
    let context = ClassifyContext {
        ref_counts,
        prod_ref_counts,
        fixture_definition_names,
        confidence_threshold,
        whitelist,
        analysis_root,
    };
    // Each definition is classified independently; the indexed collect keeps
    // the original order within every output list.
    let outcomes: Vec<Outcome> = definitions
        .into_par_iter()
        .zip(nested_under_callable)
        .zip(in_test_file)
        .zip(reachability.flags.par_iter())
        .map(|(((def, nested), is_test_file), flags)| {
            context.classify(def, nested, is_test_file, *flags)
        })
        .collect();

    for outcome in outcomes {
        let def = match outcome {
            Outcome::Skip => continue,
            Outcome::MethodWithRefs(def) => {
                result.methods_with_refs.push(def);
                continue;
            }
            Outcome::Report(def) => def,
        };
        match def.def_type {
            DefinitionType::Function => result.unused_functions.push(def),
            DefinitionType::Method => result.unused_methods.push(def),
            DefinitionType::Class => result.unused_classes.push(def),
            DefinitionType::Import => result.unused_imports.push(def),
            DefinitionType::Variable => result.unused_variables.push(def),
            DefinitionType::Parameter => result.unused_parameters.push(def),
        }
    }

    result
}
fn has_parent_callable(
    def: &Definition,
    definition_kinds: &FxHashMap<&str, DefinitionType>,
) -> bool {
    // Only nested functions and classes benefit from the parent-callable exemption;
    // methods are owned by their class and analyzed via the class container instead.
    if !matches!(
        def.def_type,
        DefinitionType::Function | DefinitionType::Class
    ) {
        return false;
    }

    let Some((parent_name, _)) = def.full_name.rsplit_once('.') else {
        return false;
    };

    let Some(parent_kind) = definition_kinds.get(parent_name) else {
        return false;
    };

    match def.def_type {
        DefinitionType::Function => matches!(
            parent_kind,
            DefinitionType::Function | DefinitionType::Method
        ),
        // Nested classes live inside another class/method/function — treat the
        // enclosing container as the unit of reachability and suppress the FP
        // when the parent is itself a structural callable/container.
        DefinitionType::Class => matches!(
            parent_kind,
            DefinitionType::Function | DefinitionType::Method | DefinitionType::Class
        ),
        _ => false,
    }
}
pub(super) fn promote_methods_from_unused_classes(
    unused_methods: &mut Vec<Definition>,
    methods_with_refs: Vec<Definition>,
    confidence_threshold: u8,
    unused_classes: &[Definition],
) {
    let unused_class_names: FxHashSet<_> = unused_classes
        .iter()
        .map(|cls| cls.full_name.as_str())
        .collect();
    let unreachable_class_names: FxHashSet<_> = unused_classes
        .iter()
        .filter(|cls| cls.is_unreachable)
        .map(|cls| cls.full_name.as_str())
        .collect();

    for def in methods_with_refs {
        // For methods whose parent class is itself unreachable, structural evidence overrides
        // the confidence threshold (same principle as `should_report_definition`).
        // Exception: confidence == 0 means explicitly suppressed — never surface those.
        let parent_is_unreachable = def
            .full_name
            .rfind('.')
            .is_some_and(|idx| unreachable_class_names.contains(&def.full_name[..idx]));
        if parent_is_unreachable {
            if def.confidence == 0 {
                continue;
            }
        } else if def.confidence == 0 || def.confidence < confidence_threshold {
            continue;
        }

        if def.simple_name.starts_with("visit_")
            || def.simple_name.starts_with("leave_")
            || def.simple_name.starts_with("transform_")
            || def.simple_name.starts_with("on_")
            || def.simple_name.starts_with("watch_")
            || def.simple_name == "compose"
        {
            continue;
        }

        if let Some(last_dot) = def.full_name.rfind('.') {
            let parent_class = &def.full_name[..last_dot];
            if unused_class_names.contains(parent_class) {
                let is_unreachable = unreachable_class_names.contains(parent_class);
                let mut method = def;
                method.is_unreachable |= is_unreachable;
                unused_methods.push(method);
            }
        }
    }
}
