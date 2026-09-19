use super::infrastructure;
use super::runtime;
use super::structural;
use super::types::Effort;
use crate::scoring::dimensions::ScoringContext;
use crate::scoring::types::DimensionScore;

/// Candidate recommendation descriptor before reduction simulation.
pub struct CandidateRecommendation {
    pub id: String,
    pub title: String,
    pub dimension: String,
    pub target_rating: u32,
    pub effort: Effort,
    pub description: String,
    pub action_steps: Vec<String>,
    pub affected_files: Vec<String>,
}

/// Collect all candidate recommendations applicable to the current codebase.
pub fn collect_candidates(
    ctx: &ScoringContext<'_>,
    dimensions: &[DimensionScore],
) -> Vec<CandidateRecommendation> {
    let mut candidates = Vec::new();

    structural::collect_structural_candidates(ctx, dimensions, &mut candidates);
    runtime::collect_runtime_candidates(ctx, dimensions, &mut candidates);
    infrastructure::collect_infrastructure_candidates(ctx, dimensions, &mut candidates);

    candidates
}
