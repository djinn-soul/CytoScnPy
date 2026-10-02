//! Candidate recommendations for runtime predictability.

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

/// Collect candidate recommendations for runtime predictability.
pub fn collect_runtime_candidates(
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

    if ctx.anti_patterns.stats.magic_numbers > 0 {
        let files: Vec<String> = ctx
            .anti_patterns
            .matches
            .iter()
            .filter(|m| m.kind == crate::anti_patterns::AntiPatternKind::MagicNumber)
            .map(|item| item.file.to_string_lossy().into_owned())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .take(5)
            .collect();
        let current = get_rating(dimensions, "Runtime predictability");
        out.push(CandidateRecommendation {
            id: "replace-magic-numbers".to_owned(),
            title: format!(
                "Extract {} magic number(s) into named constants",
                ctx.anti_patterns.stats.magic_numbers
            ),
            dimension: "Runtime predictability".to_owned(),
            target_rating: current.saturating_sub(1),
            effort: Effort::Low,
            description: "Magic numbers in conditional logic obscure domain intent and make adjustments error-prone.".to_owned(),
            action_steps: vec![
                "Define uppercase module or class constants for literal numeric thresholds.".to_owned(),
                "Replace inline numbers in comparisons and conditions with the named constants.".to_owned(),
            ],
            affected_files: files,
        });
    }

    if ctx.anti_patterns.stats.nested_callbacks > 0 {
        let files: Vec<String> = ctx
            .anti_patterns
            .matches
            .iter()
            .filter(|m| m.kind == crate::anti_patterns::AntiPatternKind::DeeplyNestedCallback)
            .map(|item| item.file.to_string_lossy().into_owned())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .take(5)
            .collect();
        let current = get_rating(dimensions, "Runtime predictability");
        out.push(CandidateRecommendation {
            id: "refactor-nested-callbacks".to_owned(),
            title: format!(
                "Flatten {} deeply nested callback/block structure(s)",
                ctx.anti_patterns.stats.nested_callbacks
            ),
            dimension: "Runtime predictability".to_owned(),
            target_rating: current.saturating_sub(1),
            effort: Effort::Medium,
            description: "Deep nesting (depth >= 4) increases cognitive burden, nesting complexity, and edge-case fragility.".to_owned(),
            action_steps: vec![
                "Extract inner callbacks and closures into standalone helper functions.".to_owned(),
                "Use early returns or guard clauses to eliminate nested branching.".to_owned(),
            ],
            affected_files: files,
        });
    }

    if ctx.singletons.stats.total > 0 {
        let files: Vec<String> = ctx
            .singletons
            .matches
            .iter()
            .map(|item| item.file.to_string_lossy().into_owned())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .take(5)
            .collect();
        let current = get_rating(dimensions, "Runtime predictability");
        out.push(CandidateRecommendation {
            id: "refactor-singletons".to_owned(),
            title: format!(
                "Refactor {} singleton pattern(s) to explicit injection",
                ctx.singletons.stats.total
            ),
            dimension: "Runtime predictability".to_owned(),
            target_rating: current.saturating_sub(1),
            effort: Effort::Medium,
            description: "Global singleton instances hinder unit isolation and create invisible temporal coupling.".to_owned(),
            action_steps: vec![
                "Pass dependencies explicitly into constructors or function arguments.".to_owned(),
                "Use dependency injection or factory patterns to manage component lifecycles.".to_owned(),
            ],
            affected_files: files,
        });
    }
}
