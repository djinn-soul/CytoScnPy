use super::ScoringContext;
use crate::scoring::types::DimensionScore;

/// Dimension 10: Context pressure (weight: 5)
///
/// Evaluates LLM navigation token load, active code surface vs frozen libraries,
/// churn-complexity hotspots, duplicate code, and unreferenced dead code.
#[must_use]
pub fn context_pressure(ctx: &ScoringContext<'_>) -> DimensionScore {
    let mut rating = 0u32;
    let budget = &ctx.context.token_budget;

    let active_bytes =
        if ctx.context.git_activity.is_git_repo && ctx.context.git_activity.active_files > 0 {
            ctx.context.git_activity.active_bytes
        } else {
            ctx.context.total_bytes
        };

    let estimated_active_tokens = (active_bytes as f64 / 3.5) as usize;

    if estimated_active_tokens > 500_000 {
        rating += 2;
    } else if estimated_active_tokens > 100_000 {
        rating += 1;
    }

    let hotspot_count = ctx.context.hotspots.len();
    if hotspot_count > 3 {
        rating += 2;
    } else if hotspot_count > 0 {
        rating += 1;
    }

    if budget.navigation_pct > 75.0 {
        rating += 1;
    }

    let dead_fn_count = ctx.unreferenced.stats.total_unreferenced_functions;
    if dead_fn_count > 5 {
        rating += 1;
    }

    rating = rating.min(5);

    let mut evidence_parts = vec![format!(
        "~{} active tokens, {:.1}% navigation load (status: {})",
        format_token_count(estimated_active_tokens),
        budget.navigation_pct,
        budget.status_verdict
    )];

    if hotspot_count > 0 {
        evidence_parts.push(format!("{hotspot_count} churn-complexity hotspots"));
    }
    if dead_fn_count > 0 {
        evidence_parts.push(format!(
            "{dead_fn_count} unreferenced large functions in isolated files"
        ));
    }
    let dup_pct = ctx.duplicates.stats.duplicate_pct;
    if dup_pct > 5.0 {
        evidence_parts.push(format!("{dup_pct:.1}% code duplication"));
    }

    let evidence = evidence_parts.join(", ");
    let weight = 5;
    DimensionScore {
        name: "Context pressure".to_owned(),
        weight,
        rating,
        raw_contribution: f64::from(weight) * (f64::from(rating) / 5.0),
        evidence,
    }
}

fn format_token_count(tokens: usize) -> String {
    if tokens >= 1_000_000 {
        format!("{:.1}M", tokens as f64 / 1_000_000.0)
    } else if tokens >= 1_000 {
        format!("{}k", tokens / 1_000)
    } else {
        tokens.to_string()
    }
}
