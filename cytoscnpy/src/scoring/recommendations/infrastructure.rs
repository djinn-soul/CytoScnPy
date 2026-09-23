//! Candidate recommendations for project infrastructure, tooling, and documentation.

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

/// Collect candidate recommendations for repository infrastructure, tooling, and documentation.
pub fn collect_infrastructure_candidates(
    ctx: &ScoringContext<'_>,
    dimensions: &[DimensionScore],
    out: &mut Vec<CandidateRecommendation>,
) {
    let Some(doc) = ctx.doctor else {
        return;
    };

    if !doc.reliability.has_formatter || !doc.reliability.has_linter {
        let mut improvement = 0;
        if !doc.reliability.has_formatter {
            improvement += 2;
        }
        if !doc.reliability.has_linter {
            improvement += 1;
        }
        let current = get_rating(dimensions, "Style consistency");
        out.push(CandidateRecommendation {
            id: "configure-formatter-linter".to_owned(),
            title: "Configure code formatter and linter (Ruff / Black)".to_owned(),
            dimension: "Style consistency".to_owned(),
            target_rating: current.saturating_sub(improvement),
            effort: Effort::Low,
            description: "Automated linting and formatting prevents style drift and lowers code review overhead.".to_owned(),
            action_steps: vec![
                "Add [tool.ruff] configuration to pyproject.toml.".to_owned(),
                "Run ruff check --fix and ruff format on the codebase.".to_owned(),
            ],
            affected_files: vec!["pyproject.toml".to_owned()],
        });
    }

    if doc.structure.test_files == 0 || doc.structure.test_to_source_ratio < 0.20 {
        let mut improvement = 0;
        if !doc.reliability.has_tests {
            improvement += 1;
        }
        if doc.structure.test_files == 0 {
            improvement += 1;
        }
        if doc.structure.test_to_source_ratio < 0.20 {
            improvement += 1;
        }
        let current = get_rating(dimensions, "Test safety net");
        out.push(CandidateRecommendation {
            id: "add-test-safety-net".to_owned(),
            title: "Expand automated test safety net (pytest)".to_owned(),
            dimension: "Test safety net".to_owned(),
            target_rating: current.saturating_sub(improvement.max(1)),
            effort: Effort::Medium,
            description: "A low test-to-source ratio leaves code refactoring without a safety net."
                .to_owned(),
            action_steps: vec![
                "Set up pytest configuration in pyproject.toml.".to_owned(),
                "Add unit tests covering critical business logic and entrypoints.".to_owned(),
            ],
            affected_files: vec!["tests/".to_owned()],
        });
    }

    if !doc.reliability.has_readme_setup {
        let has_readme = doc.configs.iter().any(|c| {
            c.category == crate::doctor::ConfigCategory::Documentation
                && c.name.to_lowercase().contains("readme")
        });
        let improvement = if has_readme { 1 } else { 2 };
        let current = get_rating(dimensions, "Documentation");
        out.push(CandidateRecommendation {
            id: "document-setup-instructions".to_owned(),
            title: "Document setup instructions in README.md".to_owned(),
            dimension: "Documentation".to_owned(),
            target_rating: current.saturating_sub(improvement),
            effort: Effort::Low,
            description: "Clear setup and quickstart documentation enables seamless developer and agent onboarding.".to_owned(),
            action_steps: vec![
                "Add installation, prerequisites, and execution commands to README.md.".to_owned(),
            ],
            affected_files: vec!["README.md".to_owned()],
        });
    }

    if !doc.reliability.has_type_checker {
        let current = get_rating(dimensions, "Style consistency");
        out.push(CandidateRecommendation {
            id: "configure-type-checker".to_owned(),
            title: "Configure static type checking (mypy / pyright)".to_owned(),
            dimension: "Style consistency".to_owned(),
            target_rating: current.saturating_sub(1),
            effort: Effort::Low,
            description: "Static type checking with mypy or pyright detects type inconsistencies, invalid calls, and missing None checks before runtime.".to_owned(),
            action_steps: vec![
                "Add [tool.mypy] or [tool.pyright] configuration to pyproject.toml.".to_owned(),
                "Add type annotations to function signatures in core modules.".to_owned(),
                "Run mypy or pyright to verify type consistency across the codebase.".to_owned(),
            ],
            affected_files: vec!["pyproject.toml".to_owned()],
        });
    }

    if !doc.reliability.has_ci {
        let current = get_rating(dimensions, "Feedback loop speed");
        out.push(CandidateRecommendation {
            id: "configure-ci-pipeline".to_owned(),
            title: "Configure CI workflow automation (.github/workflows)".to_owned(),
            dimension: "Feedback loop speed".to_owned(),
            target_rating: current.saturating_sub(1),
            effort: Effort::Low,
            description: "Continuous integration ensures tests, linters, and type checkers run automatically on every pull request.".to_owned(),
            action_steps: vec![
                "Create .github/workflows/ci.yml with test and lint steps.".to_owned(),
                "Configure workflow triggers on push and pull_request to main branch.".to_owned(),
                "Verify workflow runs and passes in GitHub Actions.".to_owned(),
            ],
            affected_files: vec![".github/workflows/ci.yml".to_owned()],
        });
    }

    let has_arch = doc.configs.iter().any(|c| {
        c.category == crate::doctor::ConfigCategory::Documentation
            && c.name.to_lowercase().contains("arch")
    });
    if !has_arch {
        let current = get_rating(dimensions, "Documentation");
        out.push(CandidateRecommendation {
            id: "document-architecture".to_owned(),
            title: "Document architecture in docs/ARCHITECTURE.md".to_owned(),
            dimension: "Documentation".to_owned(),
            target_rating: current.saturating_sub(2),
            effort: Effort::Medium,
            description: "Architecture documentation details component boundaries, data flow, and layering principles for contributors and AI agents.".to_owned(),
            action_steps: vec![
                "Create docs/ARCHITECTURE.md describing high-level design and package layout.".to_owned(),
                "Document module dependency hierarchy and core component lifecycles.".to_owned(),
            ],
            affected_files: vec!["docs/ARCHITECTURE.md".to_owned()],
        });
    }
}
