//! Per-definition usage evidence and classification.

use super::reachability::ReachabilityFlags;
use crate::analyzer::apply_heuristics;
use crate::visitor::{Definition, DefinitionType};
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

fn min_usage_score(def_type: DefinitionType) -> f64 {
    match def_type {
        DefinitionType::Function | DefinitionType::Method | DefinitionType::Class => 0.35,
        _ => 0.0,
    }
}
pub(super) enum Outcome {
    Skip,
    MethodWithRefs(Definition),
    Report(Definition),
}

pub(super) struct ClassifyContext<'a> {
    pub(super) ref_counts: &'a FxHashMap<String, usize>,
    pub(super) prod_ref_counts: &'a FxHashMap<String, usize>,
    pub(super) fixture_definition_names: &'a FxHashSet<String>,
    pub(super) confidence_threshold: u8,
    pub(super) whitelist: Option<&'a WhitelistMatcher>,
    pub(super) analysis_root: &'a std::path::Path,
}

impl ClassifyContext<'_> {
    pub(super) fn classify(
        &self,
        mut def: Definition,
        is_nested_under_callable: bool,
        is_test_file: bool,
        reachability: ReachabilityFlags,
    ) -> Outcome {
        // Check if this definition is whitelisted. Match on both the fully
        // qualified name (e.g. `module.symbol`) and the bare simple name so
        // Vulture-style whitelists (bare identifiers) keep working regardless
        // of which analysis root we inferred — that root influences how the
        // module qualifier is reconstructed.
        if let Some(matcher) = self.whitelist {
            let relative_path = def
                .file
                .strip_prefix(self.analysis_root)
                .unwrap_or(def.file.as_path());
            let normalized_path = relative_path.to_string_lossy().replace('\\', "/");
            if matcher.is_whitelisted(&def.name, Some(&normalized_path))
                || matcher.is_whitelisted(&def.simple_name, Some(&normalized_path))
            {
                return Outcome::Skip; // Skip whitelisted definitions
            }
        }

        // For production-file definitions use only prod refs so that test-only callers
        // don't mask unused prod code. For test-file definitions keep the full ref_counts
        // so that fixtures used exclusively by tests are not falsely flagged.
        let effective_refs = if is_test_file {
            self.ref_counts
        } else {
            self.prod_ref_counts
        };

        let mut evidence =
            sync_definition_reference(&mut def, effective_refs, self.fixture_definition_names);

        apply_heuristics(&mut def);

        if reachability.implicitly_used {
            evidence.implicit_method = true;
            def.references = std::cmp::max(def.references, 1);
        }

        if (def.def_type == "function" || def.def_type == "method" || def.def_type == "class")
            && !reachability.reachable
        {
            let loose_ref_exists = effective_refs.contains_key(&format!(".{}", def.simple_name));
            let captured_as_value_only = def.is_captured && !reachability.explicitly_called;

            if !loose_ref_exists && !is_nested_under_callable && !captured_as_value_only {
                def.is_unreachable = true;
                def.references = 0;
            }
        }

        if (!def.is_unreachable || evidence.has_strong_exact())
            && def.references > 0
            && evidence.score() > 0.0
            && !evidence.has_strong_exact()
            && evidence.score() < min_usage_score(def.def_type)
        {
            // Weak fallback evidence did not clear the minimum score for this symbol type.
            def.references = 0;
        }

        // A referenced definition is never reported, so move it rather than clone.
        if def.def_type == "method" && def.references > 0 {
            return Outcome::MethodWithRefs(def);
        }

        if !should_report_definition(&def, self.confidence_threshold) {
            return Outcome::Skip;
        }

        if def.is_unreachable {
            let type_label = def.def_type.as_str();
            def.message = Some(format!("Unreachable {}: `{}`", type_label, def.simple_name));
        }
        Outcome::Report(def)
    }
}
fn should_report_definition(def: &Definition, confidence_threshold: u8) -> bool {
    if def.references != 0 {
        return false;
    }

    // Structural reachability evidence should not be hidden by intent penalties.
    // Keep this constrained to executable/type container symbols to avoid surfacing
    // low-confidence variables/imports solely due heuristic interactions.
    // Exception: confidence == 0 means the item was explicitly suppressed (pragma, dunder, etc.)
    // and must never be surfaced even if the call graph marks it unreachable.
    if def.is_unreachable
        && def.confidence > 0
        && matches!(
            def.def_type,
            DefinitionType::Function | DefinitionType::Method | DefinitionType::Class
        )
    {
        return true;
    }

    // Zero stays hidden even at `--confidence 0`.
    def.confidence > 0 && def.confidence >= confidence_threshold
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

    if def.references == 0
        && (def.def_type == "method" || def.def_type == "function" || def.def_type == "class")
    {
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
