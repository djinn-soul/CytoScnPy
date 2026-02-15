use super::reachability::ReachabilityContext;
use crate::analyzer::apply_heuristics;
use crate::visitor::Definition;
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
    reachability: &ReachabilityContext,
    fixture_definition_names: &FxHashSet<String>,
    confidence_threshold: u8,
) -> ClassificationResult {
    let mut result = ClassificationResult::new();

    for mut def in definitions {
        sync_definition_reference(&mut def, ref_counts, fixture_definition_names);

        apply_heuristics(&mut def);

        if reachability
            .implicitly_used_methods
            .contains(&def.full_name)
        {
            def.references = std::cmp::max(def.references, 1);
        }

        if (def.def_type == "function" || def.def_type == "method" || def.def_type == "class")
            && !reachability.reachable_nodes.contains(&def.full_name)
        {
            let loose_ref_exists = if def.def_type == "method" || def.def_type == "function" {
                ref_counts.contains_key(&format!(".{}", def.simple_name))
            } else {
                false
            };

            if !loose_ref_exists {
                def.is_unreachable = true;
                def.references = 0;
            }
        }

        if def.def_type == "method" && def.references > 0 {
            result.methods_with_refs.push(def.clone());
        }

        if def.references != 0 || def.confidence < confidence_threshold {
            continue;
        }

        if def.is_unreachable {
            let type_label = match def.def_type.as_str() {
                "function" => "function",
                "method" => "method",
                "class" => "class",
                _ => &def.def_type,
            };
            def.message = Some(format!("Unreachable {}: `{}`", type_label, def.simple_name));
        }

        match def.def_type.as_str() {
            "function" => result.unused_functions.push(def),
            "method" => result.unused_methods.push(def),
            "class" => result.unused_classes.push(def),
            "import" => result.unused_imports.push(def),
            "variable" => result.unused_variables.push(def),
            "parameter" => result.unused_parameters.push(def),
            _ => {}
        }
    }

    result
}

fn sync_definition_reference(
    def: &mut Definition,
    ref_counts: &FxHashMap<String, usize>,
    fixture_definition_names: &FxHashSet<String>,
) {
    if let Some(count) = ref_counts.get(&def.full_name) {
        if (def.def_type == "variable" || def.def_type == "parameter")
            && !def.is_enum_member
            && *count == 0
        {
            return;
        }
        def.references = *count;
        return;
    }

    let mut matched = false;

    if def.is_enum_member {
        if let Some(dot_idx) = def.full_name.rfind('.') {
            let parent = &def.full_name[..dot_idx];
            if let Some(class_dot) = parent.rfind('.') {
                let class_member = format!("{}.{}", &parent[class_dot + 1..], def.simple_name);
                if let Some(count) = ref_counts.get(&class_member) {
                    def.references = *count;
                    matched = true;
                }
            }
        }
    }

    if !matched && !def.is_enum_member {
        let should_fallback = def.def_type != "variable"
            && def.def_type != "parameter"
            && def.def_type != "import"
            && !fixture_definition_names.contains(&def.full_name);

        if should_fallback {
            if let Some(count) = ref_counts.get(&def.simple_name) {
                def.references = *count;
            }
        }
    }

    if def.references == 0 && (def.def_type == "method" || def.def_type == "function") {
        let loose_attr_key = format!(".{}", def.simple_name);
        if let Some(count) = ref_counts.get(&loose_attr_key) {
            if *count > 0 {
                def.references = *count;
            }
        }
    }
}

pub(super) fn promote_methods_from_unused_classes(
    unused_methods: &mut Vec<Definition>,
    methods_with_refs: &[Definition],
    confidence_threshold: u8,
    unused_classes: &[Definition],
) {
    let unused_class_names: std::collections::HashSet<_> = unused_classes
        .iter()
        .map(|cls| cls.full_name.clone())
        .collect();
    let unreachable_class_names: std::collections::HashSet<_> = unused_classes
        .iter()
        .filter(|cls| cls.is_unreachable)
        .map(|cls| cls.full_name.clone())
        .collect();

    for def in methods_with_refs {
        if def.confidence < confidence_threshold {
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
                let mut method = def.clone();
                if unreachable_class_names.contains(parent_class) {
                    method.is_unreachable = true;
                }
                unused_methods.push(method);
            }
        }
    }
}
