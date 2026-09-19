//! Candidate recommendations for runtime predictability and context pressure.

use super::builder::CandidateRecommendation;
use super::types::Effort;
use crate::scoring::dimensions::ScoringContext;
use crate::scoring::types::DimensionScore;

fn get_rating(dimensions: &[DimensionScore], name: &str) -> u32 {
    dimensions
        .iter()
        .find(|d| d.name == name)
        .map_or(0, |d| d.rating)
}

/// Collect candidate recommendations for runtime and context pressure.
pub fn collect_runtime_candidates(
    ctx: &ScoringContext<'_>,
    dimensions: &[DimensionScore],
    out: &mut Vec<CandidateRecommendation>,
) {
    collect_runtime_predictability(ctx, dimensions, out);
    collect_context_pressure(ctx, dimensions, out);
}

fn collect_runtime_predictability(
    ctx: &ScoringContext<'_>,
    dimensions: &[DimensionScore],
    out: &mut Vec<CandidateRecommendation>,
) {
    if ctx.globals.stats.total_globals > 0 {
        let files: Vec<String> = ctx
            .globals
            .matches
            .iter()
            .map(|item| item.file.to_string_lossy().into_owned())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .take(5)
            .collect();
        let penalty = if ctx.globals.stats.total_globals > 20 {
            2
        } else {
            1
        };
        let current = get_rating(dimensions, "Runtime predictability");
        out.push(CandidateRecommendation {
            id: "encapsulate-mutable-globals".to_owned(),
            title: format!(
                "Encapsulate {} mutable global variable(s)",
                ctx.globals.stats.total_globals
            ),
            dimension: "Runtime predictability".to_owned(),
            target_rating: current.saturating_sub(penalty),
            effort: Effort::Low,
            description: "Module-level mutable state leaks state between invocations and hinders concurrency.".to_owned(),
            action_steps: vec![
                "Wrap module collections in configuration objects or container classes.".to_owned(),
                "Pass state explicitly to functions rather than relying on global mutations.".to_owned(),
                "Use threading.local or ContextVar if contextual state is required.".to_owned(),
            ],
            affected_files: files,
        });
    }

    if ctx.exceptions.stats.total > 0 {
        let files: Vec<String> = ctx
            .exceptions
            .matches
            .iter()
            .map(|item| item.file.to_string_lossy().into_owned())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .take(5)
            .collect();
        let penalty = 1;
        let current = get_rating(dimensions, "Runtime predictability");
        out.push(CandidateRecommendation {
            id: "replace-bare-empty-exceptions".to_owned(),
            title: format!(
                "Replace {} bare or empty exception handler(s)",
                ctx.exceptions.stats.total
            ),
            dimension: "Runtime predictability".to_owned(),
            target_rating: current.saturating_sub(penalty),
            effort: Effort::Low,
            description: "Bare and empty exception handlers swallow fatal errors and obscure bugs.".to_owned(),
            action_steps: vec![
                "Replace bare 'except:' with specific exception classes (e.g. 'except (KeyError, ValueError):').".to_owned(),
                "Add error logging or explicit degradation logic inside empty handler bodies.".to_owned(),
            ],
            affected_files: files,
        });
    }

    if ctx.side_effects.stats.total > 0 {
        let files: Vec<String> = ctx
            .side_effects
            .matches
            .iter()
            .map(|item| item.file.to_string_lossy().into_owned())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .take(5)
            .collect();
        let penalty = 1;
        let current = get_rating(dimensions, "Runtime predictability");
        out.push(CandidateRecommendation {
            id: "remove-module-side-effects".to_owned(),
            title: format!(
                "Eliminate {} import-time module side effect(s)",
                ctx.side_effects.stats.total
            ),
            dimension: "Runtime predictability".to_owned(),
            target_rating: current.saturating_sub(penalty),
            effort: Effort::Medium,
            description: "Top-level side effects (network, file I/O, event listeners) run unexpectedly during imports.".to_owned(),
            action_steps: vec![
                "Move top-level execution calls into an explicit main() or initialization function.".to_owned(),
                "Guard entrypoint execution behind if __name__ == '__main__': blocks.".to_owned(),
            ],
            affected_files: files,
        });
    }
}

fn collect_context_pressure(
    ctx: &ScoringContext<'_>,
    dimensions: &[DimensionScore],
    out: &mut Vec<CandidateRecommendation>,
) {
    if ctx.unreferenced.stats.total_unreferenced_functions > 0 {
        let files: Vec<String> = ctx
            .unreferenced
            .items
            .iter()
            .map(|f| f.file.to_string_lossy().into_owned())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .take(5)
            .collect();
        let current = get_rating(dimensions, "Context pressure");
        out.push(CandidateRecommendation {
            id: "prune-unreferenced-functions".to_owned(),
            title: format!(
                "Audit and prune {} unreferenced large function(s)",
                ctx.unreferenced.stats.total_unreferenced_functions
            ),
            dimension: "Context pressure".to_owned(),
            target_rating: current.saturating_sub(1),
            effort: Effort::Low,
            description: "Unused functions in isolated files consume LLM context window without providing value.".to_owned(),
            action_steps: vec![
                "Verify if flagged functions are external public APIs or truly dead code.".to_owned(),
                "Remove dead functions or document export requirements.".to_owned(),
            ],
            affected_files: files,
        });
    }

    if ctx.duplicates.stats.duplicate_pct > 3.0 {
        let mut dup_files = Vec::new();
        for cluster in &ctx.duplicates.clusters {
            for loc in &cluster.locations {
                let s = loc.file.to_string_lossy().into_owned();
                if !dup_files.contains(&s) {
                    dup_files.push(s);
                }
            }
            if dup_files.len() >= 5 {
                break;
            }
        }

        let current = get_rating(dimensions, "Context pressure");
        out.push(CandidateRecommendation {
            id: "deduplicate-code-clusters".to_owned(),
            title: format!(
                "Refactor {} duplicate cluster(s) ({:.1}% duplication)",
                ctx.duplicates.stats.cluster_count, ctx.duplicates.stats.duplicate_pct
            ),
            dimension: "Context pressure".to_owned(),
            target_rating: current.saturating_sub(1),
            effort: Effort::Medium,
            description: "Code duplication bloats the active context window and causes inconsistent bug fixes.".to_owned(),
            action_steps: vec![
                "Extract identical logic blocks into shared utility functions.".to_owned(),
                "Parameterize differing values across clone instances.".to_owned(),
            ],
            affected_files: dup_files,
        });
    }

    if !ctx.context.hotspots.is_empty() {
        let hotspot_files = ctx
            .context
            .hotspots
            .iter()
            .map(|h| h.display_path.clone())
            .take(5)
            .collect();

        let penalty = if ctx.context.hotspots.len() > 3 { 2 } else { 1 };
        let current = get_rating(dimensions, "Context pressure");
        out.push(CandidateRecommendation {
            id: "refactor-hotspot-files".to_owned(),
            title: format!(
                "Simplify {} churn-complexity hotspot file(s)",
                ctx.context.hotspots.len()
            ),
            dimension: "Context pressure".to_owned(),
            target_rating: current.saturating_sub(penalty),
            effort: Effort::High,
            description: "Frequently changed files with high cyclomatic complexity present the highest bug risk.".to_owned(),
            action_steps: vec![
                "Break large hotspot functions into smaller, single-purpose functions.".to_owned(),
                "Add comprehensive unit test coverage before refactoring.".to_owned(),
            ],
            affected_files: hotspot_files,
        });
    }
}
