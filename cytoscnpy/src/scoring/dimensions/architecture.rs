use super::ScoringContext;
use crate::scoring::types::DimensionScore;

/// Dimension 2: Architecture clarity (weight: 15)
///
/// Evaluates directory hierarchy depth, god modules, bidirectional package layering,
/// duplicate filenames, and function collisions.
#[must_use]
pub fn architecture_clarity(ctx: &ScoringContext<'_>) -> DimensionScore {
    let mut rating = 0u32;

    let max_depth = ctx
        .doctor
        .map_or(0, |doc| doc.structure.max_directory_depth);
    if max_depth > 8 {
        rating += 2;
    } else if max_depth > 5 {
        rating += 1;
    }

    let total_files = ctx.context.total_files;
    if total_files > 500 {
        rating += 1;
    }

    let layer_violations = ctx.architecture.stats.bidirectional_group_deps.len();
    if layer_violations > 0 {
        rating += 1;
    }

    let god_modules = ctx.architecture.stats.god_modules.len();
    if god_modules > 0 {
        rating += 1;
    }

    let duplicate_files = ctx.searchability.stats.duplicate_filenames;
    if duplicate_files > 10 {
        rating += 1;
    }

    let collisions = ctx.searchability.stats.function_name_collisions;
    if collisions > 15 {
        rating += 1;
    }

    rating = rating.min(5);

    let mut evidence_parts = vec![format!("{} files, max depth {}", total_files, max_depth)];

    if layer_violations > 0 {
        evidence_parts.push(format!("{layer_violations} bidirectional group deps"));
    }
    if god_modules > 0 {
        evidence_parts.push(format!("{god_modules} god modules"));
    }
    if duplicate_files > 0 {
        evidence_parts.push(format!("{duplicate_files} duplicate filenames"));
    }
    if collisions > 0 {
        evidence_parts.push(format!("{collisions} function name collisions"));
    }

    let evidence = evidence_parts.join(", ");
    let weight = 15;
    DimensionScore {
        name: "Architecture clarity".to_owned(),
        weight,
        rating,
        raw_contribution: f64::from(weight) * (f64::from(rating) / 5.0),
        evidence,
    }
}

/// Dimension 3: Coupling / blast radius (weight: 15)
///
/// Evaluates import graph fan-in, fan-out, and circular dependency components.
#[must_use]
pub fn coupling_blast_radius(ctx: &ScoringContext<'_>) -> DimensionScore {
    let stats = &ctx.architecture.stats;
    let mut rating = 0u32;

    if stats.avg_fan_out > 15.0 {
        rating += 2;
    } else if stats.avg_fan_out > 8.0 {
        rating += 1;
    }

    if stats.max_fan_out > 30 {
        rating += 1;
    }

    if stats.circular_dependency_count > 5 {
        rating += 2;
    } else if stats.circular_dependency_count > 0 {
        rating += 1;
    }

    rating = rating.min(5);

    let max_fan_out_label = stats
        .max_fan_out_module
        .as_deref()
        .map(|m| format!(" ({m})"))
        .unwrap_or_default();

    let max_fan_in_label = stats
        .max_fan_in_module
        .as_deref()
        .map(|m| format!(" ({m})"))
        .unwrap_or_default();

    let cycle_label = if stats.circular_dependency_count > 0 {
        format!(
            ", {} circular deps (largest: {} modules)",
            stats.circular_dependency_count, stats.largest_cycle_size
        )
    } else {
        ", 0 circular deps".to_owned()
    };

    let evidence = format!(
        "avg fan-out: {:.1}, max fan-out: {}{}, avg fan-in: {:.1}, max fan-in: {}{}{}",
        stats.avg_fan_out,
        stats.max_fan_out,
        max_fan_out_label,
        stats.avg_fan_in,
        stats.max_fan_in,
        max_fan_in_label,
        cycle_label
    );

    let weight = 15;
    DimensionScore {
        name: "Coupling / blast radius".to_owned(),
        weight,
        rating,
        raw_contribution: f64::from(weight) * (f64::from(rating) / 5.0),
        evidence,
    }
}
