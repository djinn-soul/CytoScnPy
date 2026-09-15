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
        }
    }
}
