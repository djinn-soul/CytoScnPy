use super::reachability::ReachabilityContext;
use crate::analyzer::apply_heuristics;
use crate::visitor::Definition;
use crate::whitelist::WhitelistMatcher;
use rustc_hash::{FxHashMap, FxHashSet};

const WEIGHT_FULL_NAME_REF: f64 = 1.0;
const WEIGHT_ENUM_MEMBER_REF: f64 = 1.0;
const WEIGHT_SIMPLE_FALLBACK: f64 = 0.35;
const WEIGHT_LOOSE_ATTR_FALLBACK: f64 = 0.60;
const WEIGHT_IMPLICIT_METHOD: f64 = 1.0;

#[derive(Default, Clone, Copy)]
struct UsageEvidence {
    full_name_ref: bool,
    enum_member_ref: bool,
    simple_fallback: bool,
    loose_attr_fallback: bool,
    implicit_method: bool,
}

impl UsageEvidence {
    fn has_strong_exact(self) -> bool {
        self.full_name_ref || self.enum_member_ref
    }

    fn score(self) -> f64 {
        let mut score = 0.0;
        if self.full_name_ref {
            score += WEIGHT_FULL_NAME_REF;
        }
        if self.enum_member_ref {
            score += WEIGHT_ENUM_MEMBER_REF;
        }
        if self.simple_fallback {
            score += WEIGHT_SIMPLE_FALLBACK;
        }
        if self.loose_attr_fallback {
            score += WEIGHT_LOOSE_ATTR_FALLBACK;
        }
        if self.implicit_method {
            score += WEIGHT_IMPLICIT_METHOD;
        }
        score.min(1.0)
    }
}

fn min_usage_score(def_type: &str) -> f64 {
    match def_type {
        "function" | "method" | "class" => 0.35,
        _ => 0.0,
    }
}

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
    whitelist: Option<&WhitelistMatcher>,
    analysis_root: &std::path::Path,
) -> ClassificationResult {
    let mut result = ClassificationResult::new();

    for mut def in definitions {
        // Check if this definition is whitelisted
        if let Some(matcher) = whitelist {
            let relative_path = def
                .file
                .strip_prefix(analysis_root)
                .unwrap_or(def.file.as_path());
            let normalized_path = relative_path.to_string_lossy().replace('\\', "/");
            if matcher.is_whitelisted(&def.name, Some(&normalized_path)) {
                continue; // Skip whitelisted definitions
            }
        }

        let mut evidence =
            sync_definition_reference(&mut def, ref_counts, fixture_definition_names);

        apply_heuristics(&mut def);

        if reachability
            .implicitly_used_methods
            .contains(&def.full_name)
        {
            evidence.implicit_method = true;
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

        if (!def.is_unreachable || evidence.has_strong_exact())
            && def.references > 0
            && evidence.score() > 0.0
            && !evidence.has_strong_exact()
            && evidence.score() < min_usage_score(&def.def_type)
        {
            // Weak fallback evidence did not clear the minimum score for this symbol type.
            def.references = 0;
        }

        if def.def_type == "method" && def.references > 0 {
            result.methods_with_refs.push(def.clone());
        }

        if !should_report_definition(&def, confidence_threshold) {
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

fn should_report_definition(def: &Definition, confidence_threshold: u8) -> bool {
    if def.references != 0 {
        return false;
    }

    // Structural reachability evidence should not be hidden by intent penalties.
    // Keep this constrained to executable/type container symbols to avoid surfacing
    // low-confidence variables/imports solely due heuristic interactions.
    if def.is_unreachable && matches!(def.def_type.as_str(), "function" | "method" | "class") {
        return true;
    }

    def.confidence >= confidence_threshold
}

fn sync_definition_reference(
    def: &mut Definition,
    ref_counts: &FxHashMap<String, usize>,
    fixture_definition_names: &FxHashSet<String>,
) -> UsageEvidence {
    let mut evidence = UsageEvidence::default();

    if let Some(count) = ref_counts.get(&def.full_name) {
        let is_var_like = def.def_type == "variable" || def.def_type == "parameter";
        if is_var_like && !def.is_enum_member && *count == 0 {
            return evidence;
        }
        def.references = *count;
        evidence.full_name_ref = *count > 0;
        // Do not return early for zero-count non var/param symbols.
        // They may still have valid fallback evidence (e.g. `.method_name`).
        if *count > 0 || is_var_like {
            return evidence;
        }
    }

    let mut matched = false;

    if def.is_enum_member {
        if let Some(dot_idx) = def.full_name.rfind('.') {
            let parent = &def.full_name[..dot_idx];
            if let Some(class_dot) = parent.rfind('.') {
                let class_member = format!("{}.{}", &parent[class_dot + 1..], def.simple_name);
                if let Some(count) = ref_counts.get(&class_member) {
                    def.references = *count;
                    evidence.enum_member_ref = *count > 0;
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
                evidence.simple_fallback = *count > 0;
            }
        }
    }

    if def.references == 0 && (def.def_type == "method" || def.def_type == "function") {
        let loose_attr_key = format!(".{}", def.simple_name);
        if let Some(count) = ref_counts.get(&loose_attr_key) {
            if *count > 0 {
                def.references = *count;
                evidence.loose_attr_fallback = true;
            }
        }
    }

    evidence
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
