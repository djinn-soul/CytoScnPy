use super::ScoringContext;
use crate::scoring::types::DimensionScore;

/// Dimension 4: Style consistency (weight: 10)
///
/// Evaluates formatter and linter adoption and identifier naming consistency.
#[must_use]
pub fn style_consistency(ctx: &ScoringContext<'_>) -> DimensionScore {
    let mut rating = 3u32;

    let has_formatter = ctx.doctor.is_some_and(|doc| doc.reliability.has_formatter);
    let has_linter = ctx.doctor.is_some_and(|doc| doc.reliability.has_linter);

    if has_formatter {
        rating = rating.saturating_sub(2);
    }
    if has_linter {
        rating = rating.saturating_sub(1);
    }

    let naming_ratio = ctx.naming.stats.dominant_style_ratio;
    if naming_ratio < 0.60 {
        rating = (rating + 2).min(5);
    } else if naming_ratio < 0.80 {
        rating = (rating + 1).min(5);
    }

    let config_label = match (has_formatter, has_linter) {
        (true, true) => "formatter + linter configured",
        (true, false) => "formatter configured",
        (false, true) => "linter configured",
        (false, false) => "no formatter/linter",
    };

    let evidence = format!(
        "{}, naming consistency: {:.1}% (dominant: {})",
        config_label,
        ctx.naming.stats.consistency_score(),
        ctx.naming.stats.dominant_style
    );

    let weight = 10;
    DimensionScore {
        name: "Style consistency".to_owned(),
        weight,
        rating,
        raw_contribution: f64::from(weight) * (f64::from(rating) / 5.0),
        evidence,
    }
}

/// Dimension 6: Runtime predictability (weight: 10)
///
/// Evaluates mutable global state, bare/empty exceptions, import-time side effects,
/// singletons, wildcard imports, magic numbers, and nested callbacks.
#[must_use]
pub fn runtime_predictability(ctx: &ScoringContext<'_>) -> DimensionScore {
    let mut rating = 0u32;

    let globals_count = ctx.globals.stats.total_globals;
    if globals_count > 20 {
        rating += 2;
    } else if globals_count > 0 {
        rating += 1;
    }

    let exceptions_count = ctx.exceptions.stats.total;
    if exceptions_count > 5 {
        rating += 1;
    }

    let side_effects_count = ctx.side_effects.stats.total;
    if side_effects_count > 5 {
        rating += 1;
    }

    let singletons_count = ctx.singletons.stats.total;
    if singletons_count > 3 {
        rating += 1;
    }

    let wildcards_count = ctx.wildcards.stats.total;
    if wildcards_count > 5 {
        rating += 1;
    }

    let anti_patterns_count = ctx.anti_patterns.stats.total;
    if anti_patterns_count > 10 {
        rating += 1;
    }

    rating = rating.min(5);

    let mut evidence_parts = Vec::new();
    if globals_count > 0 {
        evidence_parts.push(format!("{globals_count} global mutable states"));
    }
    if exceptions_count > 0 {
        evidence_parts.push(format!("{exceptions_count} exception anti-patterns"));
    }
    if side_effects_count > 0 {
        evidence_parts.push(format!("{side_effects_count} import-time side effects"));
    }
    if singletons_count > 0 {
        evidence_parts.push(format!("{singletons_count} singleton patterns"));
    }
    if wildcards_count > 0 {
        evidence_parts.push(format!("{wildcards_count} wildcard imports"));
    }
    if anti_patterns_count > 0 {
        evidence_parts.push(format!("{anti_patterns_count} code anti-patterns"));
    }

    let evidence = if evidence_parts.is_empty() {
        "Clean: no runtime hazards, mutable globals, or anti-patterns detected".to_owned()
    } else {
        evidence_parts.join(", ")
    };

    let weight = 10;
    DimensionScore {
        name: "Runtime predictability".to_owned(),
        weight,
        rating,
        raw_contribution: f64::from(weight) * (f64::from(rating) / 5.0),
        evidence,
    }
}
