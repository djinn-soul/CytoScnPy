//! Aggregation of multiple DoctorResult reports across scan targets.

use std::collections::HashMap;
use std::path::Path;

use super::types::{
    ConfigCategory, DetectedConfig, DoctorResult, LanguageStats, PyProjectInspection,
    RepoStructureStats, SetupReliabilityScore, SetupVerdict,
};

/// Aggregates multiple `DoctorResult` instances into a single order-independent result.
/// Returns `None` if the input slice is empty.
#[must_use]
pub fn aggregate_doctor_results(
    results: &[DoctorResult],
    root_path: &Path,
) -> Option<DoctorResult> {
    if results.is_empty() {
        return None;
    }
    if results.len() == 1 {
        let mut single = results[0].clone();
        single.root_path = root_path.to_path_buf();
        return Some(single);
    }

    // 1. Merge and deduplicate configs deterministically
    let mut config_map = HashMap::new();
    for r in results {
        for c in &r.configs {
            let key = (c.category, c.name.clone(), c.path.clone());
            config_map.entry(key).or_insert_with(|| c.clone());
        }
    }
    let mut configs: Vec<DetectedConfig> = config_map.into_values().collect();
    configs.sort_by(|a, b| {
        a.category
            .cmp(&b.category)
            .then_with(|| a.name.cmp(&b.name))
            .then_with(|| a.path.cmp(&b.path))
    });

    // 2. Merge PyProjectInspection
    let mut pyproject: Option<PyProjectInspection> = None;
    for r in results {
        if let Some(p) = &r.pyproject {
            match &mut pyproject {
                None => pyproject = Some(p.clone()),
                Some(existing) => {
                    existing.has_ruff |= p.has_ruff;
                    existing.has_black |= p.has_black;
                    existing.has_mypy |= p.has_mypy;
                    existing.has_pyright |= p.has_pyright;
                    existing.has_pytest |= p.has_pytest;
                    existing.has_pylint |= p.has_pylint;
                    existing.has_poetry |= p.has_poetry;
                    existing.has_flit_or_hatch |= p.has_flit_or_hatch;
                    if existing.build_backend.is_none() {
                        existing.build_backend.clone_from(&p.build_backend);
                    }
                    existing.dependencies_count += p.dependencies_count;
                    existing.dev_dependencies_count += p.dev_dependencies_count;
                }
            }
        }
    }

    // 3. Merge structure stats
    let total_files: usize = results.iter().map(|r| r.structure.total_files).sum();
    let total_lines: usize = results.iter().map(|r| r.structure.total_lines).sum();
    let total_bytes: u64 = results.iter().map(|r| r.structure.total_bytes).sum();
    let source_files: usize = results.iter().map(|r| r.structure.source_files).sum();
    let source_lines: usize = results.iter().map(|r| r.structure.source_lines).sum();
    let test_files: usize = results.iter().map(|r| r.structure.test_files).sum();
    let test_lines: usize = results.iter().map(|r| r.structure.test_lines).sum();
    let test_to_source_ratio = if source_lines > 0 {
        test_lines as f64 / source_lines as f64
    } else {
        0.0
    };
    let avg_file_lines = total_lines.checked_div(total_files).unwrap_or(0);
    let max_directory_depth = results
        .iter()
        .map(|r| r.structure.max_directory_depth)
        .max()
        .unwrap_or(0);

    let largest = results
        .iter()
        .max_by(|a, b| {
            a.structure
                .largest_file_lines
                .cmp(&b.structure.largest_file_lines)
                .then_with(|| {
                    b.structure
                        .largest_file_path
                        .cmp(&a.structure.largest_file_path)
                })
        })
        .map(|r| {
            (
                r.structure.largest_file_path.clone(),
                r.structure.largest_file_lines,
            )
        })
        .unwrap_or_default();

    let mut lang_map: HashMap<String, (usize, usize, u64)> = HashMap::new();
    for r in results {
        for l in &r.structure.languages {
            let entry = lang_map.entry(l.language.clone()).or_insert((0, 0, 0));
            entry.0 += l.file_count;
            entry.1 += l.line_count;
            entry.2 += l.byte_count;
        }
    }
    let mut languages: Vec<LanguageStats> = lang_map
        .into_iter()
        .map(
            |(lang, (file_count, line_count, byte_count))| LanguageStats {
                language: lang,
                file_count,
                line_count,
                byte_count,
            },
        )
        .collect();
    languages.sort_by(|a, b| {
        b.line_count
            .cmp(&a.line_count)
            .then_with(|| a.language.cmp(&b.language))
    });

    let structure = RepoStructureStats {
        total_files,
        total_lines,
        total_bytes,
        source_files,
        source_lines,
        test_files,
        test_lines,
        test_to_source_ratio,
        avg_file_lines,
        largest_file_path: largest.0,
        largest_file_lines: largest.1,
        max_directory_depth,
        languages,
    };

    // 4. Merge setup reliability
    let has_lockfile = results.iter().any(|r| r.reliability.has_lockfile)
        || configs
            .iter()
            .any(|c| c.category == ConfigCategory::Lockfile);
    let has_ci = results.iter().any(|r| r.reliability.has_ci)
        || configs.iter().any(|c| c.category == ConfigCategory::CI);
    let has_tests = results.iter().any(|r| r.reliability.has_tests)
        || configs
            .iter()
            .any(|c| c.category == ConfigCategory::TestFramework);
    let has_linter = results.iter().any(|r| r.reliability.has_linter)
        || configs.iter().any(|c| c.category == ConfigCategory::Linter);
    let has_formatter = results.iter().any(|r| r.reliability.has_formatter)
        || configs
            .iter()
            .any(|c| c.category == ConfigCategory::Formatter);
    let has_type_checker = results.iter().any(|r| r.reliability.has_type_checker)
        || configs
            .iter()
            .any(|c| c.category == ConfigCategory::TypeChecker);
    let has_docker = results.iter().any(|r| r.reliability.has_docker)
        || configs.iter().any(|c| c.category == ConfigCategory::Docker);
    let has_build_script = results.iter().any(|r| r.reliability.has_build_script)
        || configs
            .iter()
            .any(|c| c.category == ConfigCategory::BuildScript);
    let has_gitignore = results.iter().any(|r| r.reliability.has_gitignore)
        || configs
            .iter()
            .any(|c| c.category == ConfigCategory::GitIgnore);
    let has_readme_setup = results.iter().any(|r| r.reliability.has_readme_setup);

    let mut score = 0u32;
    if has_lockfile {
        score += 20;
    }
    if has_ci {
        score += 15;
    }
    if has_tests {
        score += 15;
    }
    if has_linter {
        score += 10;
    }
    if has_formatter {
        score += 10;
    }
    if has_type_checker {
        score += 10;
    }
    if has_readme_setup {
        score += 10;
    }
    if has_docker {
        score += 5;
    }
    if has_build_script {
        score += 5;
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

    let reliability = SetupReliabilityScore {
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
        recommendations: Vec::new(),
    };

    Some(DoctorResult {
        root_path: root_path.to_path_buf(),
        configs,
        pyproject,
        structure,
        reliability,
    })
}
