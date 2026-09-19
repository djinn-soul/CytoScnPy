use super::calculator::*;
use super::reporter::*;
use super::types::*;

#[test]
fn test_verdict_mapping() {
    assert_eq!(Verdict::from_index(0), Verdict::Clean);
    assert_eq!(Verdict::from_index(20), Verdict::Clean);
    assert_eq!(Verdict::from_index(21), Verdict::Acceptable);
    assert_eq!(Verdict::from_index(40), Verdict::Acceptable);
    assert_eq!(Verdict::from_index(41), Verdict::Messy);
    assert_eq!(Verdict::from_index(60), Verdict::Messy);
    assert_eq!(Verdict::from_index(61), Verdict::Sloppy);
    assert_eq!(Verdict::from_index(80), Verdict::Sloppy);
    assert_eq!(Verdict::from_index(81), Verdict::Disaster);
    assert_eq!(Verdict::from_index(100), Verdict::Disaster);

    assert!(Verdict::Clean.is_passing());
    assert!(Verdict::Acceptable.is_passing());
    assert!(!Verdict::Messy.is_passing());
    assert!(!Verdict::Sloppy.is_passing());
    assert!(!Verdict::Disaster.is_passing());
}

#[test]
fn test_compute_size_multiplier() {
    // 0 bytes -> 0.0 multiplier
    assert!((compute_size_multiplier(0, None, 176_000) - 0.0).abs() < f64::EPSILON);

    // Active bytes used over total bytes when available
    let mult_active = compute_size_multiplier(10_000_000, Some(50_000), 176_000);
    let mult_total = compute_size_multiplier(50_000, None, 176_000);
    assert!((mult_active - mult_total).abs() < 1e-6);

    // Large codebase caps at 1.0
    let mult_huge = compute_size_multiplier(100_000_000, None, 176_000);
    assert!((mult_huge - 1.0).abs() < f64::EPSILON);
}

#[test]
fn test_calculate_raw_score_and_slop_index() {
    let dimensions = vec![
        DimensionScore {
            name: "Setup reliability".to_owned(),
            weight: 10,
            rating: 1, // 10 * 0.2 = 2.0
            raw_contribution: 2.0,
            evidence: "1 missing item".to_owned(),
        },
        DimensionScore {
            name: "Architecture clarity".to_owned(),
            weight: 15,
            rating: 2, // 15 * 0.4 = 6.0
            raw_contribution: 6.0,
            evidence: "some layer violations".to_owned(),
        },
    ];

    let raw = calculate_raw_score(&dimensions);
    assert!((raw - 8.0).abs() < f64::EPSILON);

    // With 0.5 size multiplier: 8.0 * 0.5 = 4
    let index = calculate_slop_index(raw, 0.5);
    assert_eq!(index, 4);
}

#[test]
fn test_simulate_reduction() {
    let dimensions = vec![DimensionScore {
        name: "Architecture clarity".to_owned(),
        weight: 15,
        rating: 4, // 15 * (4/5) = 12.0
        raw_contribution: 12.0,
        evidence: "layer violations".to_owned(),
    }];

    // Improving rating from 4 to 1:
    // old contrib = 12.0, new contrib = 15 * 0.2 = 3.0, delta = 9.0
    // with multiplier 1.0: delta = 9
    let points_saved = simulate_reduction(&dimensions, "Architecture clarity", 1, 1.0);
    assert_eq!(points_saved, 9);

    // No savings if proposed rating >= current rating
    assert_eq!(
        simulate_reduction(&dimensions, "Architecture clarity", 4, 1.0),
        0
    );
    assert_eq!(
        simulate_reduction(&dimensions, "Architecture clarity", 5, 1.0),
        0
    );
}

#[test]
fn test_finalize_score_result_gates() {
    let dimensions = vec![DimensionScore {
        name: "Setup reliability".to_owned(),
        weight: 10,
        rating: 2,
        raw_contribution: 4.0,
        evidence: "missing lockfile".to_owned(),
    }];

    // Clean pass
    let res = finalize_score_result(
        dimensions.clone(),
        Vec::new(),
        1.0,
        &ScoringOptions {
            max_score: Some(10),
            fail_on_any: false,
            ci: false,
            ..ScoringOptions::default()
        },
    );
    assert!(res.passed_gate);
    assert_eq!(res.slop_index, 4);
    assert_eq!(res.verdict, Verdict::Clean);

    // Fails on max_score
    let res_max_fail = finalize_score_result(
        dimensions.clone(),
        Vec::new(),
        1.0,
        &ScoringOptions {
            max_score: Some(2),
            ..ScoringOptions::default()
        },
    );
    assert!(!res_max_fail.passed_gate);
    assert!(res_max_fail.failure_reason.unwrap().contains("exceeds"));

    // Fails on fail_on_any
    let res_any_fail = finalize_score_result(
        dimensions,
        Vec::new(),
        1.0,
        &ScoringOptions {
            fail_on_any: true,
            ..ScoringOptions::default()
        },
    );
    assert!(!res_any_fail.passed_gate);
    assert!(res_any_fail.failure_reason.unwrap().contains("fail-on-any"));
}

#[test]
fn test_ci_with_explicit_max_score_gate() {
    // Dimension producing slop_index = 45 (verdict Messy)
    let dimensions = vec![DimensionScore {
        name: "Setup reliability".to_owned(),
        weight: 100,
        rating: 2,
        raw_contribution: 45.0,
        evidence: "evidence".to_owned(),
    }];

    // --ci --max-score 100 on score 45: MUST PASS
    let res_ci_max = finalize_score_result(
        dimensions.clone(),
        Vec::new(),
        1.0,
        &ScoringOptions {
            max_score: Some(100),
            ci: true,
            ..ScoringOptions::default()
        },
    );
    assert!(res_ci_max.passed_gate);
    assert_eq!(res_ci_max.slop_index, 45);

    // --ci alone on score 45 (verdict Messy > 40): MUST FAIL
    let res_ci_only = finalize_score_result(
        dimensions,
        Vec::new(),
        1.0,
        &ScoringOptions {
            max_score: None,
            ci: true,
            ..ScoringOptions::default()
        },
    );
    assert!(!res_ci_only.passed_gate);
    assert!(res_ci_only
        .failure_reason
        .unwrap()
        .contains("CI gate failed"));
}

#[test]
fn test_documentation_recommendation_reduction() {
    let dimensions = vec![DimensionScore {
        name: "Documentation".to_owned(),
        weight: 10,
        rating: 5,
        raw_contribution: 10.0,
        evidence: "no README, no arch docs".to_owned(),
    }];

    // Target rating 3 (improving by 2 for README setup without arch/contrib):
    let reduction = simulate_reduction(&dimensions, "Documentation", 3, 1.0);
    // Old contrib: 10, new contrib: 10 * 0.6 = 6, delta = 4
    assert_eq!(reduction, 4);
}

#[test]
fn test_terminal_report_format() {
    let result = ScoreResult {
        slop_index: 15,
        raw_score: 18.5,
        size_multiplier: 0.81,
        verdict: Verdict::Clean,
        dimensions: vec![DimensionScore {
            name: "Setup reliability".to_owned(),
            weight: 10,
            rating: 1,
            raw_contribution: 2.0,
            evidence: "All signals present".to_owned(),
        }],
        recommendations: vec![super::recommendations::Recommendation {
            id: "break-circular-dependencies".to_owned(),
            title: "Break 1 circular import cycle".to_owned(),
            dimension: "Coupling / blast radius".to_owned(),
            estimated_reduction: 3,
            target_rating: 0,
            effort: super::recommendations::Effort::Medium,
            description: "Cycle found".to_owned(),
            action_steps: vec!["Refactor".to_owned()],
            affected_files: vec!["a.py".to_owned(), "b.py".to_owned()],
        }],
        passed_gate: true,
        failure_reason: None,
    };

    let text = format_terminal_report(&result);
    assert!(text.contains("DESLOPIFY SLOP INDEX REPORT"));
    assert!(text.contains("15 / 100"));
    assert!(text.contains("[CLEAN"));
    assert!(text.contains("Setup reliability"));
    assert!(text.contains("Top Prioritized Remediations:"));
    assert!(text.contains("Break 1 circular import cycle"));
    assert!(text.contains("[GATE PASSED]"));
}

#[test]
fn test_llm_report_format() {
    let result = ScoreResult {
        slop_index: 25,
        raw_score: 30.0,
        size_multiplier: 0.85,
        verdict: Verdict::Acceptable,
        dimensions: vec![],
        recommendations: vec![super::recommendations::Recommendation {
            id: "encapsulate-mutable-globals".to_owned(),
            title: "Encapsulate 2 mutable global variable(s)".to_owned(),
            dimension: "Runtime predictability".to_owned(),
            estimated_reduction: 4,
            target_rating: 0,
            effort: super::recommendations::Effort::Low,
            description: "Globals found in config".to_owned(),
            action_steps: vec!["Wrap in dataclass".to_owned()],
            affected_files: vec!["config.py".to_owned()],
        }],
        passed_gate: true,
        failure_reason: None,
    };

    let llm_md = super::recommendations::format_llm_report(&result);
    assert!(llm_md.contains("# Codebase Remediation Plan (DeSlopify)"));
    assert!(llm_md.contains("25 / 100"));
    assert!(llm_md.contains("ACCEPTABLE"));
    assert!(llm_md.contains("Encapsulate 2 mutable global variable(s)"));
    assert!(llm_md.contains("config.py"));
    assert!(llm_md.contains("Wrap in dataclass"));
}
