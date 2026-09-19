use super::ScoringContext;
use crate::doctor::ConfigCategory;
use crate::scoring::types::DimensionScore;

/// Dimension 1: Setup reliability (weight: 10)
///
/// Evaluates presence of package managers, lockfiles, containerization,
/// build scripts, and ignore rules.
#[must_use]
pub fn setup_reliability(ctx: &ScoringContext<'_>) -> DimensionScore {
    let mut rating = 5u32;
    let mut evidence_parts = Vec::new();

    if let Some(doc) = ctx.doctor {
        let has_dep = doc
            .configs
            .iter()
            .any(|c| c.category == ConfigCategory::DependencyManager);
        let has_lock = doc.reliability.has_lockfile;
        let has_docker = doc.reliability.has_docker;
        let has_build = doc.reliability.has_build_script;
        let has_ignore = doc.reliability.has_gitignore;

        if has_dep {
            rating = rating.saturating_sub(1);
        } else {
            evidence_parts.push("no dependency manager".to_owned());
        }

        if has_lock {
            rating = rating.saturating_sub(1);
        } else {
            evidence_parts.push("no lockfile".to_owned());
        }

        if has_docker {
            rating = rating.saturating_sub(1);
            evidence_parts.push("Docker configured".to_owned());
        } else {
            evidence_parts.push("no Docker config".to_owned());
        }

        if has_build {
            rating = rating.saturating_sub(1);
            evidence_parts.push("build script present".to_owned());
        }

        if has_ignore {
            rating = rating.saturating_sub(1);
        } else {
            evidence_parts.push("no .gitignore".to_owned());
        }
    } else {
        evidence_parts.push("no repository tooling scan data".to_owned());
    }

    let evidence = if evidence_parts.is_empty() {
        "All setup signals present and valid".to_owned()
    } else {
        evidence_parts.join(", ")
    };

    let weight = 10;
    DimensionScore {
        name: "Setup reliability".to_owned(),
        weight,
        rating,
        raw_contribution: f64::from(weight) * (f64::from(rating) / 5.0),
        evidence,
    }
}

/// Dimension 5: Test safety net (weight: 15)
///
/// Evaluates test file volume, test-to-source ratio, and test framework configuration.
#[must_use]
pub fn test_safety_net(ctx: &ScoringContext<'_>) -> DimensionScore {
    let mut rating = 4u32;
    let (test_ratio, test_count, source_count, has_framework) = match ctx.doctor {
        Some(doc) => (
            doc.structure.test_to_source_ratio,
            doc.structure.test_files,
            doc.structure.source_files,
            doc.reliability.has_tests,
        ),
        None => (0.0, 0, ctx.context.total_files, false),
    };

    if test_ratio > 0.50 {
        rating = rating.saturating_sub(2);
    } else if test_ratio > 0.20 {
        rating = rating.saturating_sub(1);
    }

    if has_framework {
        rating = rating.saturating_sub(1);
    }

    if test_count > 0 {
        rating = rating.saturating_sub(1);
    }

    let framework_label = if has_framework {
        "test framework configured"
    } else {
        "no test framework config"
    };

    let evidence = format!(
        "test ratio: {test_ratio:.2} ({test_count} test files / {source_count} source files), {framework_label}"
    );

    let weight = 15;
    DimensionScore {
        name: "Test safety net".to_owned(),
        weight,
        rating,
        raw_contribution: f64::from(weight) * (f64::from(rating) / 5.0),
        evidence,
    }
}

/// Dimension 7: Feedback loop speed (weight: 5)
///
/// Estimates build and test feedback latency from codebase volume and automation tooling.
#[must_use]
pub fn feedback_loop_speed(ctx: &ScoringContext<'_>) -> DimensionScore {
    let mut rating = 0u32;
    let total_lines = ctx.context.total_lines;
    // Heuristic: ~2000 lines/sec throughput for compile/typecheck/lint checks
    let est_seconds = total_lines as f64 / 2000.0;

    if est_seconds > 120.0 {
        rating += 3;
    } else if est_seconds > 30.0 {
        rating += 2;
    } else if est_seconds > 5.0 {
        rating += 1;
    }

    let has_test_runner = ctx.doctor.is_some_and(|doc| doc.reliability.has_tests);
    let has_ci = ctx.doctor.is_some_and(|doc| doc.reliability.has_ci);
    let has_build = ctx
        .doctor
        .is_some_and(|doc| doc.reliability.has_build_script);

    if !has_test_runner {
        rating += 1;
    }
    if !has_ci && !has_build {
        rating += 1;
    }

    rating = rating.min(5);

    let est_label = if est_seconds > 60.0 {
        format!("~{:.0}min", est_seconds / 60.0)
    } else {
        format!("~{est_seconds:.1}s")
    };

    let evidence = format!(
        "estimated feedback: {}, {}, {}",
        est_label,
        if has_test_runner {
            "test runner configured"
        } else {
            "no test runner"
        },
        if has_ci {
            "CI configured"
        } else if has_build {
            "build script present"
        } else {
            "no CI or build automation"
        }
    );

    let weight = 5;
    DimensionScore {
        name: "Feedback loop speed".to_owned(),
        weight,
        rating,
        raw_contribution: f64::from(weight) * (f64::from(rating) / 5.0),
        evidence,
    }
}

/// Dimension 8: Documentation (weight: 10)
///
/// Evaluates repository documentation, setup instructions, architecture notes, and contributing guidelines.
#[must_use]
pub fn documentation(ctx: &ScoringContext<'_>) -> DimensionScore {
    let mut rating = 5u32;
    let mut evidence_parts = Vec::new();

    if let Some(doc) = ctx.doctor {
        let has_readme = doc.configs.iter().any(|c| {
            c.category == ConfigCategory::Documentation && c.name.to_lowercase().contains("readme")
        });
        let has_readme_setup = doc.reliability.has_readme_setup;
        let has_arch = doc.configs.iter().any(|c| {
            c.category == ConfigCategory::Documentation && c.name.to_lowercase().contains("arch")
        });
        let has_contrib = doc.configs.iter().any(|c| {
            c.category == ConfigCategory::Documentation && c.name.to_lowercase().contains("contrib")
        });

        if has_readme && has_readme_setup {
            rating = rating.saturating_sub(2);
            evidence_parts.push("README with setup instructions".to_owned());
        } else if has_readme {
            rating = rating.saturating_sub(1);
            evidence_parts.push("README present (missing setup instructions)".to_owned());
        } else {
            evidence_parts.push("no README".to_owned());
        }

        if has_arch {
            rating = rating.saturating_sub(2);
            evidence_parts.push("architecture docs present".to_owned());
        } else {
            evidence_parts.push("no architecture docs".to_owned());
        }

        if has_contrib {
            rating = rating.saturating_sub(1);
            evidence_parts.push("contributing guide present".to_owned());
        }
    } else {
        evidence_parts.push("documentation uninspected".to_owned());
    }

    let evidence = evidence_parts.join(", ");
    let weight = 10;
    DimensionScore {
        name: "Documentation".to_owned(),
        weight,
        rating,
        raw_contribution: f64::from(weight) * (f64::from(rating) / 5.0),
        evidence,
    }
}

/// Dimension 9: Dependency boundaries (weight: 5)
///
/// Evaluates lockfile discipline, ignore rules, and vendor/generated code isolation.
#[must_use]
pub fn dependency_boundaries(ctx: &ScoringContext<'_>) -> DimensionScore {
    let mut rating = 2u32;
    let mut evidence_parts = Vec::new();

    if let Some(doc) = ctx.doctor {
        if doc.reliability.has_gitignore {
            rating = rating.saturating_sub(1);
            evidence_parts.push(".gitignore present".to_owned());
        } else {
            evidence_parts.push("no .gitignore".to_owned());
        }

        if doc.reliability.has_lockfile {
            rating = rating.saturating_sub(1);
            evidence_parts.push("lockfile present".to_owned());
        } else {
            evidence_parts.push("no lockfile".to_owned());
        }
    } else {
        evidence_parts.push("no doctor data".to_owned());
    }

    let evidence = evidence_parts.join(", ");
    let weight = 5;
    DimensionScore {
        name: "Dependency boundaries".to_owned(),
        weight,
        rating,
        raw_contribution: f64::from(weight) * (f64::from(rating) / 5.0),
        evidence,
    }
}
