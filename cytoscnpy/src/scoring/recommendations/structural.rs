//! Candidate recommendations for architecture, coupling, and searchability.

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

/// Collect candidate recommendations for architecture, coupling, and searchability.
pub fn collect_structural_candidates(
    ctx: &ScoringContext<'_>,
    dimensions: &[DimensionScore],
    out: &mut Vec<CandidateRecommendation>,
) {
    collect_architecture(ctx, dimensions, out);
    collect_coupling(ctx, dimensions, out);
    collect_searchability(ctx, dimensions, out);
}

fn collect_architecture(
    ctx: &ScoringContext<'_>,
    dimensions: &[DimensionScore],
    out: &mut Vec<CandidateRecommendation>,
) {
    if !ctx.architecture.stats.god_modules.is_empty() {
        let god_files = ctx
            .architecture
            .stats
            .god_modules
            .iter()
            .map(|m| m.file_path.to_string_lossy().into_owned())
            .collect();

        let current = get_rating(dimensions, "Architecture clarity");
        out.push(CandidateRecommendation {
            id: "decompose-god-modules".to_owned(),
            title: format!(
                "Decompose {} god module(s)",
                ctx.architecture.stats.god_modules.len()
            ),
            dimension: "Architecture clarity".to_owned(),
            target_rating: current.saturating_sub(1),
            effort: Effort::High,
            description: "God modules have excessive incoming dependencies and accumulate multiple unrelated responsibilities.".to_owned(),
            action_steps: vec![
                "Identify distinct responsibilities inside each god module.".to_owned(),
                "Extract cohesive submodules into a subpackage with clear public exports.".to_owned(),
                "Update dependent modules to import from specific submodules.".to_owned(),
            ],
            affected_files: god_files,
        });
    }

    if !ctx.architecture.stats.bidirectional_group_deps.is_empty() {
        let current = get_rating(dimensions, "Architecture clarity");
        out.push(CandidateRecommendation {
            id: "enforce-module-layering".to_owned(),
            title: format!(
                "Enforce layered architecture across {} package pair(s)",
                ctx.architecture.stats.bidirectional_group_deps.len()
            ),
            dimension: "Architecture clarity".to_owned(),
            target_rating: current.saturating_sub(1),
            effort: Effort::Medium,
            description:
                "Cross-package cycles break architectural layering and create tangled dependencies."
                    .to_owned(),
            action_steps: vec![
                "Define a strict dependency hierarchy (e.g. models -> services -> cli).".to_owned(),
                "Invert inverted dependencies using dependency injection or callbacks.".to_owned(),
                "Extract common interface models into a lower-tier foundation module.".to_owned(),
            ],
            affected_files: ctx
                .architecture
                .stats
                .bidirectional_group_deps
                .iter()
                .map(|d| format!("{} <-> {}", d.group_a, d.group_b))
                .collect(),
        });
    }
}

fn collect_coupling(
    ctx: &ScoringContext<'_>,
    dimensions: &[DimensionScore],
    out: &mut Vec<CandidateRecommendation>,
) {
    if ctx.architecture.stats.circular_dependency_count > 0 {
        let mut cycle_files = Vec::new();
        for cycle in &ctx.architecture.stats.cycles {
            for path in &cycle.file_paths {
                let s = path.to_string_lossy().into_owned();
                if !cycle_files.contains(&s) {
                    cycle_files.push(s);
                }
            }
        }
        let penalty = if ctx.architecture.stats.circular_dependency_count > 5 {
            2
        } else {
            1
        };
        let current = get_rating(dimensions, "Coupling / blast radius");
        out.push(CandidateRecommendation {
            id: "break-circular-dependencies".to_owned(),
            title: format!(
                "Break {} circular import cycle(s)",
                ctx.architecture.stats.circular_dependency_count
            ),
            dimension: "Coupling / blast radius".to_owned(),
            target_rating: current.saturating_sub(penalty),
            effort: Effort::Medium,
            description: "Circular imports degrade Python runtime predictability and impair static analysis reasoning.".to_owned(),
            action_steps: vec![
                "Move shared data types or interfaces to a leaf module.".to_owned(),
                "Convert top-level mutual imports into TYPE_CHECKING guards or local imports.".to_owned(),
                "Verify import order with cytoscnpy graph --cycles .".to_owned(),
            ],
            affected_files: cycle_files,
        });
    }
}

fn collect_searchability(
    ctx: &ScoringContext<'_>,
    dimensions: &[DimensionScore],
    out: &mut Vec<CandidateRecommendation>,
) {
    if ctx.searchability.stats.duplicate_filenames > 0 {
        let dup_files = ctx
            .searchability
            .duplicate_files
            .iter()
            .map(|d| d.filename.clone())
            .take(5)
            .collect();

        let current = get_rating(dimensions, "Architecture clarity");
        out.push(CandidateRecommendation {
            id: "disambiguate-duplicate-filenames".to_owned(),
            title: format!(
                "Disambiguate {} duplicate filename(s)",
                ctx.searchability.stats.duplicate_filenames
            ),
            dimension: "Architecture clarity".to_owned(),
            target_rating: current.saturating_sub(1),
            effort: Effort::Low,
            description: "Identically named files across directories cause agent hallucinations and path errors.".to_owned(),
            action_steps: vec![
                "Rename generic files (e.g. types.py, utils.py) to domain-specific names (e.g. user_types.py).".to_owned(),
                "Update import statements to reflect the renamed files.".to_owned(),
            ],
            affected_files: dup_files,
        });
    }
}
