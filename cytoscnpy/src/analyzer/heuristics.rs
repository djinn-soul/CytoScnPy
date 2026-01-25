//! Heuristics for adjusting confidence scores on definitions.

use crate::constants::{AUTO_CALLED, PENALTIES};
use crate::framework::FrameworkAwareVisitor;
use crate::test_utils::TestAwareVisitor;
use crate::utils::Suppression;
use crate::visitor::Definition;
use rustc_hash::FxHashMap;

/// Applies penalty-based confidence adjustments to definitions.
///
/// This function lowers confidence for:
/// - Ignored lines (pragma: no cytoscnpy).
/// - Test files and test-decorated functions.
/// - Framework decorations (lowers confidence for framework-managed code).
/// - Private naming conventions (lowers confidence for internal helpers).
/// - Dunder methods (ignores magic methods).
#[allow(clippy::implicit_hasher)]
pub fn apply_penalties(
    def: &mut Definition,
    fv: &FrameworkAwareVisitor,
    tv: &TestAwareVisitor,
    // Map of ignored lines to their suppression type.
    // Uses `FxHashMap` (Rustc's fast hash map) for performance optimization,
    // as strict cryptographic security is not needed for integer line keys and small datasets.
    ignored_lines: &FxHashMap<usize, Suppression>,
    include_tests: bool,
) {
    // Pragma: no cytoscnpy (highest priority - always skip)
    // If the line is marked to be ignored, set confidence to 0.
    if let Some(suppression) = ignored_lines.get(&def.line) {
        // Use `matches!` for conciseness: we only need to check the variant type
        // and don't need to extract any inner data for the `All` case.
        if matches!(suppression, Suppression::All) {
            def.confidence = 0;
            return;
        }
    }

    // Test files: confidence 0 (ignore)
    // We don't want to report unused code in test files usually.
    if !include_tests && (tv.is_test_file || tv.test_decorated_lines.contains(&def.line)) {
        def.confidence = def
            .confidence
            .saturating_sub(*PENALTIES().get("test_related").unwrap_or(&100));
        if def.confidence == 0 {
            return;
        }
    }

    // Framework decorated: confidence 0 (ignore) or lower
    // Frameworks often use dependency injection or reflection, making static analysis hard.
    if fv.framework_decorated_lines.contains(&def.line) {
        def.confidence = *PENALTIES().get("framework_magic").unwrap_or(&40); // Low confidence
    }

    // Framework managed scope (e.g. inside a decorated function)
    // Variables here might be used for debugging or framework side-effects
    if def.is_framework_managed {
        def.confidence = def
            .confidence
            .saturating_sub(*PENALTIES().get("framework_managed").unwrap_or(&50));
    }

    // Mixin penalty: Methods in *Mixin classes are often used implicitly
    if def.def_type == "method" && def.full_name.contains("Mixin") {
        def.confidence = def
            .confidence
            .saturating_sub(*PENALTIES().get("mixin_class").unwrap_or(&60));
    }

    // Base/Abstract/Interface penalty
    // These are often overrides or interfaces with implicit usage.
    if def.def_type == "method"
        && (def.full_name.contains(".Base")
            || def.full_name.contains("Base")
            || def.full_name.contains("Abstract")
            || def.full_name.contains("Interface"))
    {
        def.confidence = def
            .confidence
            .saturating_sub(*PENALTIES().get("base_abstract_interface").unwrap_or(&50));
    }

    // Adapter penalty
    // Adapters are also often used implicitly, but we want to be less aggressive than Base/Abstract
    // to avoid false negatives on dead adapter methods (regression fix).
    if def.def_type == "method" && def.full_name.contains("Adapter") {
        def.confidence = def
            .confidence
            .saturating_sub(*PENALTIES().get("adapter_class").unwrap_or(&30));
    }

    // Framework lifecycle methods
    if def.def_type == "method" || def.def_type == "function" {
        if def.simple_name.starts_with("on_") || def.simple_name.starts_with("watch_") {
            def.confidence = def
                .confidence
                .saturating_sub(*PENALTIES().get("lifecycle_hook").unwrap_or(&30));
        }
        if def.simple_name == "compose" {
            def.confidence = def
                .confidence
                .saturating_sub(*PENALTIES().get("compose_method").unwrap_or(&40));
        }
    }

    // Private names
    // Names starting with _ are often internal and might not be used externally,
    // but might be used implicitly. We lower confidence.
    if def.simple_name.starts_with('_') && !def.simple_name.starts_with("__") {
        def.confidence = def
            .confidence
            .saturating_sub(*PENALTIES().get("private_name").unwrap_or(&80));
    }

    // Dunder methods
    // Magic methods like __init__, __str__ are called by Python internals.
    if def.simple_name.starts_with("__") && def.simple_name.ends_with("__") {
        def.confidence = def
            .confidence
            .saturating_sub(*PENALTIES().get("dunder_or_magic").unwrap_or(&100));
    }

    // Auto-called methods
    if AUTO_CALLED().contains(def.simple_name.as_str()) {
        def.confidence = def
            .confidence
            .saturating_sub(*PENALTIES().get("dunder_or_magic").unwrap_or(&100));
    }

    // Module-level constants
    if def.is_constant {
        def.confidence = def
            .confidence
            .saturating_sub(*PENALTIES().get("module_constant").unwrap_or(&80));
    }

    // In __init__.py
    if def.file.file_name().is_some_and(|n| n == "__init__.py") {
        def.confidence = def
            .confidence
            .saturating_sub(*PENALTIES().get("in_init_file").unwrap_or(&15));
    }

    // Note: TYPE_CHECKING import penalty moved to apply_heuristics()
    // because it needs def.references to be accurate (after cross-file merge)
}

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
    }

    // 3. TYPE_CHECKING imports: only suppress if actually USED in annotations
    // This runs after reference counts are merged, so def.references is accurate
    // If a TYPE_CHECKING import has 0 references, it's genuinely unused and should be reported
    if def.is_type_checking && def.def_type == "import" && def.references > 0 {
        def.confidence = def
            .confidence
            .saturating_sub(*PENALTIES().get("type_checking_import").unwrap_or(&100));
    }
}
