//! Gate checks and failure evaluation for unified `DeSlopify` analysis.

use serde::Serialize;

#[derive(Serialize)]
pub(super) struct GateSummary {
    pub passed: bool,
    pub failures: Vec<GateFailure>,
}

#[derive(Serialize)]
pub(super) struct GateFailure {
    pub check: &'static str,
    pub actual: String,
    pub limit: String,
}

pub(super) fn collect_failures(
    architecture: &crate::architecture::ArchitectureGraphResult,
    context: &crate::context::ContextAnalysisResult,
    health: &[crate::doctor::DoctorResult],
    searchability: &crate::searchability::SearchabilityResult,
    naming: &crate::naming::NamingDistributionResult,
    todos: &crate::todos::TodosResult,
    globals: &crate::globals::GlobalsResult,
    exceptions: &crate::exceptions::ExceptionsResult,
    wildcards: &crate::wildcards::WildcardsResult,
    side_effects: &crate::side_effects::SideEffectsResult,
    singletons: &crate::singletons::SingletonsResult,
    anti_patterns: &crate::anti_patterns::AntiPatternsResult,
    duplicates: &crate::duplicates::DuplicatesResult,
    unreferenced: &crate::unreferenced::UnreferencedResult,
    scoring: &crate::scoring::ScoreResult,
    config: &crate::config::Config,
    fail_on_any: bool,
) -> Vec<GateFailure> {
    let mut failures = Vec::new();
    if !fail_on_any {
        return failures;
    }

    let deslop = &config.cytoscnpy.deslop;
    push_over(
        &mut failures,
        "circular_dependencies",
        architecture.stats.circular_dependency_count,
        deslop.max_cycles,
    );
    push_over(
        &mut failures,
        "god_modules",
        architecture.stats.god_modules.len(),
        deslop.max_god_modules,
    );
    let severe_hotspots = context
        .hotspots
        .iter()
        .filter(|item| {
            matches!(
                item.risk_level,
                crate::context::HotspotRiskLevel::High | crate::context::HotspotRiskLevel::Critical
            )
        })
        .count();
    push_over(
        &mut failures,
        "severe_hotspots",
        severe_hotspots,
        deslop.max_hotspots,
    );
    for result in health {
        if result.reliability.score < deslop.min_health_score {
            failures.push(failure(
                "health_score",
                result.reliability.score.to_string(),
                format!(">= {}", deslop.min_health_score),
            ));
        }
    }
    if let Some(limit) = deslop.max_navigation_pct {
        if context.token_budget.navigation_pct > limit {
            failures.push(failure(
                "navigation_pct",
                format!("{:.1}", context.token_budget.navigation_pct),
                format!("<= {limit:.1}"),
            ));
        }
    }
    if let Some(limit) = deslop.max_duplicate_filenames {
        push_over(
            &mut failures,
            "duplicate_filenames",
            searchability.stats.duplicate_filenames,
            limit,
        );
    }
    if let Some(limit) = deslop.max_function_collisions {
        push_over(
            &mut failures,
            "function_collisions",
            searchability.stats.function_name_collisions,
            limit,
        );
    }
    if let Some(limit) = deslop.min_naming_consistency {
        if !naming.is_consistent(limit) {
            let norm_limit = crate::naming::types::normalize_consistency_threshold(limit);
            failures.push(failure(
                "naming_consistency",
                format!("{:.1}%", naming.stats.consistency_score()),
                format!(">= {:.1}%", norm_limit * 100.0),
            ));
        }
    }
    if let Some(limit) = deslop.max_todos {
        push_over(&mut failures, "todos", todos.stats.total, limit);
    }
    if let Some(limit) = deslop.max_global_mutables {
        push_over(
            &mut failures,
            "global_mutables",
            globals.stats.total_globals,
            limit,
        );
    }
    if let Some(limit) = deslop.max_bare_excepts {
        push_over(
            &mut failures,
            "bare_excepts",
            exceptions.stats.bare_except_count,
            limit,
        );
    }
    if let Some(limit) = deslop.max_empty_handlers {
        push_over(
            &mut failures,
            "empty_handlers",
            exceptions.stats.empty_handler_count,
            limit,
        );
    }
    if let Some(limit) = deslop.max_wildcard_imports {
        push_over(
            &mut failures,
            "wildcard_imports",
            wildcards.stats.total,
            limit,
        );
    }
    if let Some(limit) = deslop.max_side_effects {
        push_over(
            &mut failures,
            "side_effects",
            side_effects.stats.total,
            limit,
        );
    }
    if let Some(limit) = deslop.max_singletons {
        push_over(&mut failures, "singletons", singletons.stats.total, limit);
    }
    if let Some(limit) = deslop.max_anti_patterns {
        push_over(
            &mut failures,
            "anti_patterns",
            anti_patterns.stats.total,
            limit,
        );
    }
    if let Some(limit) = deslop.max_magic_numbers {
        push_over(
            &mut failures,
            "magic_numbers",
            anti_patterns.stats.magic_numbers,
            limit,
        );
    }
    if let Some(limit) = deslop.max_nested_callbacks {
        push_over(
            &mut failures,
            "nested_callbacks",
            anti_patterns.stats.nested_callbacks,
            limit,
        );
    }
    if let Some(limit) = deslop.max_duplicate_clusters {
        push_over(
            &mut failures,
            "duplicate_clusters",
            duplicates.stats.cluster_count,
            limit,
        );
    }
    if let Some(limit) = deslop.max_duplicate_lines {
        push_over(
            &mut failures,
            "duplicate_lines",
            duplicates.stats.total_duplicate_lines,
            limit,
        );
    }
    if let Some(limit) = deslop.max_duplicate_pct {
        if duplicates.stats.duplicate_pct > limit {
            failures.push(failure(
                "duplicate_pct",
                format!("{:.1}%", duplicates.stats.duplicate_pct),
                format!("<= {limit:.1}%"),
            ));
        }
    }
    if let Some(limit) = deslop.max_unreferenced_functions {
        push_over(
            &mut failures,
            "unreferenced_functions",
            unreferenced.stats.total_unreferenced_functions,
            limit,
        );
    }
    if let Some(limit) = deslop.max_unreferenced_lines {
        push_over(
            &mut failures,
            "unreferenced_lines",
            unreferenced.stats.total_unreferenced_lines,
            limit,
        );
    }
    if let Some(limit) = deslop.max_slop_index {
        if scoring.slop_index > limit {
            failures.push(failure(
                "slop_index",
                scoring.slop_index.to_string(),
                format!("<= {limit}"),
            ));
        }
    }
    failures
}

fn push_over(failures: &mut Vec<GateFailure>, check: &'static str, actual: usize, limit: usize) {
    if actual > limit {
        failures.push(failure(check, actual.to_string(), format!("<= {limit}")));
    }
}

fn failure(check: &'static str, actual: String, limit: String) -> GateFailure {
    GateFailure {
        check,
        actual,
        limit,
    }
}
