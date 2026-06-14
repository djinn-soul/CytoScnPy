//! Penalty-based confidence adjustments for dead-code definitions.

use crate::constants::{AUTO_CALLED, PENALTIES};
use crate::framework::FrameworkAwareVisitor;
use crate::test_utils::TestAwareVisitor;
use crate::utils::Suppression;
use crate::visitor::Definition;

/// Applies penalty-based confidence adjustments to definitions.
///
/// This lowers confidence for suppressions, tests, framework-managed code,
/// implicit naming conventions, dynamic scopes, and benign constants.
pub fn apply_penalties<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
    def: &mut Definition,
    fv: &FrameworkAwareVisitor,
    tv: &TestAwareVisitor,
    ignored_lines: &std::collections::HashMap<usize, Suppression, S1>,
    include_tests: bool,
    dynamic_scopes: &std::collections::HashSet<String, S2>,
    module_name: &str,
) {
    if is_fully_suppressed(def, ignored_lines) {
        def.confidence = 0;
        return;
    }

    apply_test_penalty(def, tv, include_tests);
    if def.confidence == 0 {
        return;
    }

    apply_framework_penalties(def, fv);
    apply_implicit_method_penalties(def);
    apply_name_penalties(def);
    apply_dynamic_scope_penalty(def, dynamic_scopes, module_name);
    apply_constant_penalty(def);
    apply_variable_boost(def);
    apply_init_file_penalty(def);

    // Boosters can push confidence above the normal maximum.
    def.confidence = def.confidence.min(100);
}

fn is_fully_suppressed<S: std::hash::BuildHasher>(
    def: &Definition,
    ignored_lines: &std::collections::HashMap<usize, Suppression, S>,
) -> bool {
    ignored_lines
        .get(&def.line)
        .is_some_and(|suppression| matches!(suppression, Suppression::All))
}

fn apply_test_penalty(def: &mut Definition, tv: &TestAwareVisitor, include_tests: bool) {
    if include_tests {
        return;
    }

    let is_fixture = tv.fixture_decorated_lines.contains(&def.line);
    let is_general_test = tv.is_test_file || tv.test_decorated_lines.contains(&def.line);
    if !is_fixture && !is_general_test {
        return;
    }

    let penalty = if is_fixture {
        *PENALTIES().get("test_decorator").unwrap_or(&30)
    } else {
        *PENALTIES().get("test_related").unwrap_or(&100)
    };
    let final_penalty = if tv.is_test_file && !is_fixture {
        100
    } else {
        penalty
    };

    def.confidence = def.confidence.saturating_sub(final_penalty);
}

fn apply_framework_penalties(def: &mut Definition, fv: &FrameworkAwareVisitor) {
    if fv.framework_decorated_lines.contains(&def.line) {
        def.confidence = *PENALTIES().get("framework_magic").unwrap_or(&40);
    }

    if def.is_framework_managed {
        subtract_penalty(def, "framework_managed", 50);
    }
}

fn apply_implicit_method_penalties(def: &mut Definition) {
    if def.def_type != "method" {
        return;
    }

    if def.full_name.contains("Mixin") {
        subtract_penalty(def, "mixin_class", 60);
    }
    if is_base_like_name(&def.full_name) {
        subtract_penalty(def, "base_abstract_interface", 50);
    }
    if def.full_name.contains("Adapter") {
        subtract_penalty(def, "adapter_class", 30);
    }
}

fn is_base_like_name(full_name: &str) -> bool {
    full_name.contains(".Base")
        || full_name.contains("Base")
        || full_name.contains("Abstract")
        || full_name.contains("Interface")
}

fn apply_name_penalties(def: &mut Definition) {
    if def.def_type == "method" || def.def_type == "function" {
        apply_lifecycle_penalty(def);
    }
    if def.simple_name.starts_with('_') && !def.simple_name.starts_with("__") {
        subtract_penalty(def, "private_name", 80);
    }
    if is_dunder_name(&def.simple_name) || AUTO_CALLED().contains(def.simple_name.as_str()) {
        subtract_penalty(def, "dunder_or_magic", 100);
    }
}

fn apply_lifecycle_penalty(def: &mut Definition) {
    if def.simple_name.starts_with("on_") || def.simple_name.starts_with("watch_") {
        subtract_penalty(def, "lifecycle_hook", 30);
    }
    if def.simple_name == "compose" {
        subtract_penalty(def, "compose_method", 40);
    }
}

fn is_dunder_name(simple_name: &str) -> bool {
    simple_name.starts_with("__") && simple_name.ends_with("__")
}

fn apply_dynamic_scope_penalty<S: std::hash::BuildHasher>(
    def: &mut Definition,
    dynamic_scopes: &std::collections::HashSet<String, S>,
    module_name: &str,
) {
    if dynamic_scopes.is_empty() {
        return;
    }

    if dynamic_scopes.contains(module_name) {
        def.confidence = def.confidence.saturating_sub(60);
        return;
    }

    if dynamic_scopes
        .iter()
        .any(|scope| is_child_scope(&def.full_name, scope))
    {
        def.confidence = def.confidence.saturating_sub(50);
    }
}

fn is_child_scope(full_name: &str, scope: &str) -> bool {
    full_name.starts_with(scope)
        && full_name.len() > scope.len()
        && full_name.as_bytes()[scope.len()] == b'.'
}

fn apply_constant_penalty(def: &mut Definition) {
    if !def.is_constant {
        return;
    }

    let base_penalty = *PENALTIES().get("module_constant").unwrap_or(&15);
    let extra_penalty = if is_config_like_constant(&def.simple_name) {
        25
    } else {
        0
    };
    def.confidence = def.confidence.saturating_sub(base_penalty + extra_penalty);
}

fn is_config_like_constant(simple_name: &str) -> bool {
    simple_name.contains("CONFIG")
        || simple_name.contains("SETTING")
        || simple_name.contains("OPTION")
        || simple_name.contains("FLAG")
        || simple_name.contains("DEFAULT")
        || simple_name.ends_with("_ENV")
}

fn apply_variable_boost(def: &mut Definition) {
    if def.def_type != "variable" {
        return;
    }

    if is_debug_like_variable(&def.simple_name) {
        def.confidence = def.confidence.saturating_add(15);
    } else if is_short_public_variable(&def.simple_name) {
        def.confidence = def.confidence.saturating_add(10);
    }
}

fn is_debug_like_variable(simple_name: &str) -> bool {
    simple_name.contains("temp")
        || simple_name.contains("tmp")
        || simple_name.contains("foo")
        || simple_name.contains("bar")
        || simple_name.starts_with("debug")
}

fn is_short_public_variable(simple_name: &str) -> bool {
    simple_name.len() <= 2 && !simple_name.starts_with('_')
}

fn apply_init_file_penalty(def: &mut Definition) {
    if def.file.file_name().is_some_and(|n| n == "__init__.py") {
        subtract_penalty(def, "in_init_file", 15);
    }
}

fn subtract_penalty(def: &mut Definition, penalty_name: &str, default: u8) {
    def.confidence = def
        .confidence
        .saturating_sub(*PENALTIES().get(penalty_name).unwrap_or(&default));
}
