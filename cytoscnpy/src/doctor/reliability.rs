use std::fs;
use std::path::Path;

use super::types::{ConfigCategory, DetectedConfig, SetupReliabilityScore, SetupVerdict};

fn check_readme_setup_instructions(repo_root: &Path) -> bool {
    let candidates = ["README.md", "README.rst", "README.txt", "README"];
    for name in candidates {
        let path = repo_root.join(name);
        if let Ok(content) = fs::read_to_string(&path) {
            let lower = content.to_lowercase();
            let keywords = [
                "install",
                "setup",
                "getting started",
                "quick start",
                "quickstart",
                "usage",
                "how to run",
                "how to build",
                "pip install",
                "cargo build",
                "npm install",
            ];
            if keywords.iter().any(|kw| lower.contains(kw)) {
                return true;
            }
        }
    }
    false
}

/// Evaluates the setup reliability score (0-100) based on repository tooling and configuration signals.
pub fn evaluate_setup_reliability(
    repo_root: &Path,
    configs: &[DetectedConfig],
) -> SetupReliabilityScore {
    let has_lockfile = configs
        .iter()
        .any(|c| c.category == ConfigCategory::Lockfile);
    let has_ci = configs.iter().any(|c| c.category == ConfigCategory::CI);
    let has_tests = configs
        .iter()
        .any(|c| c.category == ConfigCategory::TestFramework);
    let has_linter = configs.iter().any(|c| c.category == ConfigCategory::Linter);
    let has_formatter = configs
        .iter()
        .any(|c| c.category == ConfigCategory::Formatter);
    let has_type_checker = configs
        .iter()
        .any(|c| c.category == ConfigCategory::TypeChecker);
    let has_docker = configs.iter().any(|c| c.category == ConfigCategory::Docker);
    let has_build_script = configs
        .iter()
        .any(|c| c.category == ConfigCategory::BuildScript);
    let has_gitignore = configs
        .iter()
        .any(|c| c.category == ConfigCategory::GitIgnore);
    let has_readme_setup = check_readme_setup_instructions(repo_root);

    let mut score = 0u32;
    let mut recommendations = Vec::new();

    if has_lockfile {
        score += 20;
    } else {
        recommendations.push("Add a deterministic lockfile (e.g. `uv lock` or `poetry.lock`) to guarantee reproducible environments.".to_owned());
    }

    if has_ci {
        score += 15;
    } else {
        recommendations.push("Add CI automation (e.g. GitHub Actions in `.github/workflows/`) to validate code on every PR.".to_owned());
    }

    if has_tests {
        score += 15;
    } else {
        recommendations.push("Configure a test framework (e.g. pytest under `[tool.pytest]` or `pytest.ini`) and test directory.".to_owned());
    }

    if has_linter {
        score += 10;
    } else {
        recommendations.push("Add a static code linter (e.g. Ruff: `ruff.toml` or `[tool.ruff]`) to catch code smells early.".to_owned());
    }

    if has_formatter {
        score += 10;
    } else {
        recommendations.push("Configure an automated code formatter (e.g. Ruff format or Black) for uniform code style.".to_owned());
    }

    if has_type_checker {
        score += 10;
    } else {
        recommendations.push("Configure a static type checker (e.g. mypy or pyright) to prevent subtle runtime type bugs.".to_owned());
    }

    if has_readme_setup {
        score += 10;
    } else {
        recommendations.push("Add clear setup and quickstart instructions to README.md for developer and LLM agent onboarding.".to_owned());
    }

    if has_docker {
        score += 5;
    } else {
        recommendations.push("Consider adding a Dockerfile or compose setup for containerized reproducible execution.".to_owned());
    }

    if has_build_script {
        score += 5;
    }

    if !has_gitignore {
        recommendations.push("Add a `.gitignore` file to prevent committing caches, virtual environments, and secrets.".to_owned());
    }

    let verdict = if score >= 90 {
        SetupVerdict::Ready
    } else if score >= 70 {
        SetupVerdict::Solid
    } else if score >= 45 {
        SetupVerdict::Incomplete
    } else {
        SetupVerdict::AtRisk
    };

    SetupReliabilityScore {
        score,
        verdict,
        has_lockfile,
        has_ci,
        has_tests,
        has_linter,
        has_formatter,
        has_type_checker,
        has_docker,
        has_readme_setup,
        has_build_script,
        has_gitignore,
        recommendations,
    }
}
