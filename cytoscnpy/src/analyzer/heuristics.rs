//! Heuristics for adjusting confidence scores on definitions.

use crate::constants::PENALTIES;
use crate::visitor::Definition;

/// Apply advanced heuristics to definitions to reduce false positives.
pub fn apply_heuristics(def: &mut Definition) {
    // 1. Settings/Config Class Heuristic
    // If a variable is in a class ending with "Settings" or "Config" and is uppercase, ignore it.
    if def.def_type == "variable" && def.full_name.contains('.') {
        if let Some((class_part, var_name)) = def.full_name.rsplit_once('.') {
            // Check if variable is uppercase (convention for constants/settings)
            if var_name.chars().all(|c| c.is_uppercase() || c == '_') {
                // Extract simple class name
                let class_simple = class_part.split('.').next_back().unwrap_or("");
                if class_simple == "Settings"
                    || class_simple == "Config"
                    || class_simple.ends_with("Settings")
                    || class_simple.ends_with("Config")
                {
                    def.confidence = 0;
                }
            }
        }
    }

    // 2. Visitor Pattern Heuristic
    // Methods starting with "visit_", "leave_", or "transform_" are often dynamically called.
    if def.def_type == "method"
        && (def.simple_name.starts_with("visit_")
            || def.simple_name.starts_with("leave_")
            || def.simple_name.starts_with("transform_"))
    {
        // Mark as used by incrementing references
        def.references += 1;
        // Mark as root by setting confidence to 0 (immune to reachability zero-out)
        def.confidence = 0;
    }

    // 3. TYPE_CHECKING imports: only suppress if actually USED in annotations
    // This runs after reference counts are merged, so def.references is accurate
    // If a TYPE_CHECKING import has 0 references, it's genuinely unused and should be reported
    if def.is_type_checking && def.def_type == "import" && def.references > 0 {
        def.confidence = def
            .confidence
            .saturating_sub(*PENALTIES().get("type_checking_import").unwrap_or(&100));
    }

    update_category(def);
}

/// Updates the confidence category based on the final confidence score and other factors.
fn update_category(def: &mut crate::visitor::Definition) {
    use crate::visitor::UnusedCategory;

    // Special case: Config-like constants
    if def.is_constant && def.confidence < 90 {
        // If it was penalized heavily for being config-like, mark it as such
        let upper = def.simple_name.to_ascii_uppercase();
        if upper.contains("CONFIG")
            || upper.contains("SETTING")
            || upper.contains("OPTION")
            || upper.contains("FLAG")
            || upper.contains("DEFAULT")
            || upper.ends_with("_ENV")
        {
            def.category = UnusedCategory::ConfigurationConstant;
            return;
        }
    }

    def.category = match def.confidence {
        90..=100 => UnusedCategory::DefinitelyUnused,
        60..=89 => UnusedCategory::ProbablyUnused,
        // 40..=59 and fallback for very low confidence (e.g. if threshold is 0)
        _ => UnusedCategory::PossiblyIntentional,
    };
}
