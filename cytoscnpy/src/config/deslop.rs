use serde::Deserialize;

/// Additional limits applied by `deslop --fail-on-any`.
/// These limits apply only to DeSlopify-derived architecture, context, and health checks.
#[derive(Debug, Deserialize, Clone)]
#[serde(default, deny_unknown_fields)]
pub struct DeslopConfig {
    /// Maximum allowed circular dependency components.
    pub max_cycles: usize,
    /// Maximum allowed god modules.
    pub max_god_modules: usize,
    /// Maximum allowed high or critical hotspots.
    pub max_hotspots: usize,
    /// Minimum repository setup score (0-100).
    pub min_health_score: u32,
    /// Optional maximum percentage of context consumed by navigation.
    pub max_navigation_pct: Option<f64>,
    /// Optional maximum allowed duplicate filenames.
    pub max_duplicate_filenames: Option<usize>,
    /// Optional maximum allowed function name collisions.
    pub max_function_collisions: Option<usize>,
    /// Optional minimum naming style consistency ratio (0.0 - 1.0 or 0 - 100).
    pub min_naming_consistency: Option<f64>,
    /// Optional maximum allowed total TODO/FIXME/HACK/XXX and debug-print annotations.
    pub max_todos: Option<usize>,
    /// Optional maximum allowed mutable global state instances.
    pub max_global_mutables: Option<usize>,
    /// Optional maximum allowed bare-except blocks.
    pub max_bare_excepts: Option<usize>,
    /// Optional maximum allowed empty exception handlers.
    pub max_empty_handlers: Option<usize>,
    /// Optional maximum allowed wildcard imports (`from module import *`).
    pub max_wildcard_imports: Option<usize>,
    /// Optional maximum allowed module-level side effects.
    pub max_side_effects: Option<usize>,
    /// Optional maximum allowed singleton patterns.
    pub max_singletons: Option<usize>,
    /// Optional maximum allowed anti-patterns.
    pub max_anti_patterns: Option<usize>,
    /// Optional maximum allowed magic numbers.
    pub max_magic_numbers: Option<usize>,
    /// Optional maximum allowed deeply nested callbacks.
    pub max_nested_callbacks: Option<usize>,
    /// Optional maximum allowed duplicate-code clusters.
    pub max_duplicate_clusters: Option<usize>,
    /// Optional maximum allowed non-overlapping duplicate lines.
    pub max_duplicate_lines: Option<usize>,
    /// Optional maximum allowed duplicate code percentage (0.0 - 100.0).
    pub max_duplicate_pct: Option<f64>,
    /// Optional maximum allowed unreferenced large functions in isolated files.
    pub max_unreferenced_functions: Option<usize>,
    /// Optional maximum allowed lines in unreferenced large functions.
    pub max_unreferenced_lines: Option<usize>,
}

impl Default for DeslopConfig {
    fn default() -> Self {
        Self {
            max_cycles: 0,
            max_god_modules: 0,
            max_hotspots: 0,
            min_health_score: 70,
            max_navigation_pct: None,
            max_duplicate_filenames: None,
            max_function_collisions: None,
            min_naming_consistency: None,
            max_todos: None,
            max_global_mutables: None,
            max_bare_excepts: None,
            max_empty_handlers: None,
            max_wildcard_imports: None,
            max_side_effects: None,
            max_singletons: None,
            max_anti_patterns: None,
            max_magic_numbers: None,
            max_nested_callbacks: None,
            max_duplicate_clusters: None,
            max_duplicate_lines: None,
            max_duplicate_pct: None,
            max_unreferenced_functions: None,
            max_unreferenced_lines: None,
        }
    }
}
