//! Prioritized recommendations engine for codebase remediation.

mod builder;
mod infrastructure;
pub mod llm;
mod prioritizer;
mod runtime;
mod structural;
pub mod types;

pub use llm::format_llm_report;
pub use types::{Effort, Recommendation};

use crate::scoring::dimensions::ScoringContext;
use crate::scoring::types::DimensionScore;

/// Generate top prioritized recommendations from analysis context and dimension ratings.
#[must_use]
pub fn generate_recommendations(
    ctx: &ScoringContext<'_>,
    dimensions: &[DimensionScore],
    size_multiplier: f64,
) -> Vec<Recommendation> {
    let candidates = builder::collect_candidates(ctx, dimensions);
    prioritizer::prioritize_recommendations(candidates, dimensions, size_multiplier, 10)
}
