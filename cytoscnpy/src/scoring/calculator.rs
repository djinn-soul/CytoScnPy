use super::types::{DimensionScore, ScoreResult, ScoringOptions, Verdict};

/// Compute the effective size multiplier [0.0, 1.0] based on tokens vs context capacity.
///
/// When Git data is available and active files > 0, active surface bytes are used
/// rather than total bytes (frozen code functions like stable library code that the
/// model reads but does not churn).
#[must_use]
pub fn compute_size_multiplier(
    total_bytes: u64,
    active_bytes: Option<u64>,
    context_budget: usize,
) -> f64 {
    let effective_bytes = active_bytes.unwrap_or(total_bytes);
    if effective_bytes == 0 {
        return 0.0;
    }
    // Average character-to-token ratio across Python and code: ~3.5 chars/token
    let estimated_tokens = effective_bytes as f64 / 3.5;
    let budget = context_budget.max(1) as f64;
    let context_ratio = estimated_tokens / budget;
    context_ratio.powf(1.5).min(1.0)
}

/// Calculate the raw score from a list of dimension scores.
///
/// If the weights total exceeds 100, contributions are normalized to the 0-100 scale.
#[must_use]
pub fn calculate_raw_score(dimensions: &[DimensionScore]) -> f64 {
    let total_weight: u32 = dimensions.iter().map(|d| d.weight).sum();
    let sum: f64 = dimensions.iter().map(|d| d.raw_contribution).sum();
    if total_weight > 100 {
        (sum / f64::from(total_weight)) * 100.0
    } else {
        sum
    }
}

/// Calculate the final integer Slop Index from raw score and size multiplier.
#[must_use]
pub fn calculate_slop_index(raw_score: f64, size_multiplier: f64) -> u32 {
    let adjusted = raw_score * size_multiplier;
    (adjusted.round() as u32).min(100)
}

/// Simulate the score reduction (in index points) if a dimension rating is reduced.
#[must_use]
pub fn simulate_reduction(
    dimensions: &[DimensionScore],
    dimension_name: &str,
    proposed_rating: u32,
    size_multiplier: f64,
) -> u32 {
    let Some(dim) = dimensions.iter().find(|d| d.name == dimension_name) else {
        return 0;
    };
    if proposed_rating >= dim.rating {
        return 0;
    }
    let total_weight: u32 = dimensions.iter().map(|d| d.weight).sum();
    let old_contrib = dim.raw_contribution;
    let new_contrib = f64::from(dim.weight) * (f64::from(proposed_rating) / 5.0);
    let delta_raw = if total_weight > 100 {
        ((old_contrib - new_contrib) / f64::from(total_weight)) * 100.0
    } else {
        old_contrib - new_contrib
    };
    let delta = delta_raw * size_multiplier;
    delta.round() as u32
}

/// Construct a `ScoreResult` from computed dimensions and options.
#[must_use]
pub fn finalize_score_result(
    dimensions: Vec<DimensionScore>,
    recommendations: Vec<super::recommendations::Recommendation>,
    size_multiplier: f64,
    options: &ScoringOptions,
) -> ScoreResult {
    let raw_score = calculate_raw_score(&dimensions);
    let slop_index = calculate_slop_index(raw_score, size_multiplier);
    let verdict = Verdict::from_index(slop_index);

    let mut passed_gate = true;
    let mut failure_reason = None;

    if options.fail_on_any {
        let non_zero = dimensions.iter().filter(|d| d.rating > 0).count();
        if non_zero > 0 {
            passed_gate = false;
            failure_reason = Some(format!(
                "Failed --fail-on-any: {non_zero} dimension(s) have non-zero slop ratings"
            ));
        }
    }

    if let Some(max_score) = options.max_score {
        if slop_index > max_score {
            passed_gate = false;
            failure_reason = Some(format!(
                "Slop index {slop_index} exceeds configured maximum allowed score of {max_score}"
            ));
        }
    }

    if options.ci && options.max_score.is_none() && !verdict.is_passing() {
        passed_gate = false;
        if failure_reason.is_none() {
            failure_reason = Some(format!(
                "CI gate failed: verdict '{verdict}' is not passing (requires Clean or Acceptable, slop index <= 40)"
            ));
        }
    }

    ScoreResult {
        slop_index,
        raw_score,
        size_multiplier,
        verdict,
        dimensions,
        recommendations,
        passed_gate,
        failure_reason,
    }
}
