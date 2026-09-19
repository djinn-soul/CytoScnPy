mod architecture;
mod context;
mod infrastructure;
mod quality;

use crate::scoring::types::DimensionScore;

/// Unified context struct passed to all 10 dimension calculators.
pub struct ScoringContext<'a> {
    /// Module architecture and dependency graph results.
    pub architecture: &'a crate::architecture::ArchitectureGraphResult,
    /// Git-aware code churn, hotspots, and LLM token budget results.
    pub context: &'a crate::context::ContextAnalysisResult,
    /// Optional repository tooling, documentation, and setup health inspection.
    pub doctor: Option<&'a crate::doctor::DoctorResult>,
    /// Searchability, name collisions, and generic identifier results.
    pub searchability: &'a crate::searchability::SearchabilityResult,
    /// Python naming style distribution and consistency results.
    pub naming: &'a crate::naming::NamingDistributionResult,
    /// TODO, FIXME, HACK, debug print, and commented-out code scan results.
    pub todos: &'a crate::todos::TodosResult,
    /// Mutable global state and function `global` mutation results.
    pub globals: &'a crate::globals::GlobalsResult,
    /// Bare except and empty exception handler anti-pattern results.
    pub exceptions: &'a crate::exceptions::ExceptionsResult,
    /// Wildcard import (`from x import *`) results.
    pub wildcards: &'a crate::wildcards::WildcardsResult,
    /// Module-level import-time side-effect results.
    pub side_effects: &'a crate::side_effects::SideEffectsResult,
    /// Singleton anti-pattern results.
    pub singletons: &'a crate::singletons::SingletonsResult,
    /// Magic numbers and deeply nested callback anti-pattern results.
    pub anti_patterns: &'a crate::anti_patterns::AntiPatternsResult,
    /// Duplicate-code clusters and non-overlapping line results.
    pub duplicates: &'a crate::duplicates::DuplicatesResult,
    /// Potentially unreferenced large functions in isolated files results.
    pub unreferenced: &'a crate::unreferenced::UnreferencedResult,
}

/// Compute all 10 `DeSlopify` dimensions using the unified repository analysis results.
#[must_use]
pub fn compute_all(ctx: &ScoringContext<'_>) -> Vec<DimensionScore> {
    vec![
        infrastructure::setup_reliability(ctx),
        architecture::architecture_clarity(ctx),
        architecture::coupling_blast_radius(ctx),
        quality::style_consistency(ctx),
        infrastructure::test_safety_net(ctx),
        quality::runtime_predictability(ctx),
        infrastructure::feedback_loop_speed(ctx),
        infrastructure::documentation(ctx),
        infrastructure::dependency_boundaries(ctx),
        context::context_pressure(ctx),
    ]
}
