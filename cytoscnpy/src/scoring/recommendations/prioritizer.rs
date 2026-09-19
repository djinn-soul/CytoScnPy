//! Prioritizes candidate recommendations by simulated score reduction and effort.

use super::builder::CandidateRecommendation;
use super::types::Recommendation;
use crate::scoring::calculator::simulate_reduction;
use crate::scoring::types::DimensionScore;

/// Prioritize and rank candidate recommendations, returning at most `limit` items.
pub fn prioritize_recommendations(
    candidates: Vec<CandidateRecommendation>,
    dimensions: &[DimensionScore],
    size_multiplier: f64,
    limit: usize,
) -> Vec<Recommendation> {
    let mut prioritized = Vec::new();

    for cand in candidates {
        let Some(dim) = dimensions.iter().find(|d| d.name == cand.dimension) else {
            continue;
        };

        // If the dimension is already pristine (rating 0) or offers no improvement, skip
        if dim.rating == 0 || cand.target_rating >= dim.rating {
            continue;
        }

        let simulated = simulate_reduction(
            dimensions,
            &cand.dimension,
            cand.target_rating,
            size_multiplier,
        );
        // Ensure at least 1 point reduction is displayed for active issues if adjusted score > 0
        let estimated_reduction = if simulated == 0 && size_multiplier > 0.05 {
            1
        } else {
            simulated
        };

        prioritized.push(Recommendation {
            id: cand.id,
            title: cand.title,
            dimension: cand.dimension,
            estimated_reduction,
            target_rating: cand.target_rating,
            effort: cand.effort,
            description: cand.description,
            action_steps: cand.action_steps,
            affected_files: cand.affected_files,
        });
    }

    // Sort by estimated reduction descending, then effort ascending (Low < Medium < High)
    prioritized.sort_by(|a, b| {
        b.estimated_reduction
            .cmp(&a.estimated_reduction)
            .then_with(|| a.effort.cmp(&b.effort))
    });

    prioritized.truncate(limit);
    prioritized
}
