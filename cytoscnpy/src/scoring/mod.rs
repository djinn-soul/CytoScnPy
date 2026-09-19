//! Weighted Slop Index, verdict bands, and repository health scoring model.

#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]

/// Mathematical formulas, size multiplier scaling, and score reductions.
pub mod calculator;
/// The 10 `DeSlopify` scoring dimension evaluators.
pub mod dimensions;
/// Prioritized remediation recommendations engine and LLM reporting.
pub mod recommendations;
/// Human-readable ASCII terminal tables and JSON reporting.
pub mod reporter;
/// Repository summary, language breakdown, test/source counts, and configuration inventory.
pub mod summary;
#[cfg(test)]
mod tests;
/// Core data types, verdict bands, and score models.
pub mod types;

pub use calculator::{
    calculate_raw_score, calculate_slop_index, compute_size_multiplier, finalize_score_result,
    simulate_reduction,
};
pub use dimensions::{compute_all, ScoringContext};
pub use recommendations::{format_llm_report, Effort, Recommendation};
pub use reporter::{format_terminal_report, print_json_report, print_terminal_report};
pub use summary::{LanguageBreakdown, RepoSummary};
pub use types::{DimensionScore, ScoreResult, ScoringOptions, Verdict};

/// Compute the complete `ScoreResult` from all repository analysis artifacts.
#[must_use]
pub fn score_repository(ctx: &ScoringContext<'_>, options: &ScoringOptions) -> ScoreResult {
    let dimensions = compute_all(ctx);

    let active_bytes = if !options.no_git
        && ctx.context.git_activity.is_git_repo
        && ctx.context.git_activity.active_files > 0
    {
        Some(ctx.context.git_activity.active_bytes)
    } else {
        None
    };

    let size_multiplier = compute_size_multiplier(
        ctx.context.total_bytes,
        active_bytes,
        options.context_budget,
    );

    let recommendations =
        recommendations::generate_recommendations(ctx, &dimensions, size_multiplier);

    let mut result = finalize_score_result(dimensions, recommendations, size_multiplier, options);
    if let Some(doc) = ctx.doctor {
        result.summary = Some(RepoSummary::from_doctor(doc));
    }
    result
}
