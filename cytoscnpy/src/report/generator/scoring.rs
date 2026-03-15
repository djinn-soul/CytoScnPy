#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]

use crate::analyzer::AnalysisResult;
use crate::report::templates::{CategoryScore, FormattedHalsteadMetrics, OverallScore};
use crate::rules::Finding;

pub(super) fn build_halstead_view(result: &AnalysisResult) -> FormattedHalsteadMetrics {
    let file_count = result.analysis_summary.total_files.max(1) as f64;
    let h_metrics = &result.analysis_summary.halstead_metrics;

    let avg_vol = h_metrics.volume / file_count;
    let avg_diff = h_metrics.difficulty / file_count;
    let avg_effort = h_metrics.effort / file_count;
    let total_bugs = h_metrics.bugs;

    let (vol_level, vol_color, vol_icon) = if avg_vol < 1000.0 {
        ("Low", "var(--success)", "✓")
    } else {
        ("Very High", "var(--severity-high)", "⚠️")
    };
    let (diff_level, diff_color, diff_icon) = if avg_diff < 5.0 {
        ("Low", "var(--success)", "✓")
    } else if avg_diff < 10.0 {
        ("Moderate", "var(--warning)", "!")
    } else if avg_diff < 20.0 {
        ("High", "var(--severity-medium)", "⚡")
    } else {
        ("Very High", "var(--severity-high)", "⚠️")
    };
    let (eff_level, eff_color, eff_icon) = if avg_effort < 10000.0 {
        ("Low", "var(--success)", "✓")
    } else if avg_effort < 30000.0 {
        ("Moderate", "var(--warning)", "!")
    } else if avg_effort < 50000.0 {
        ("High", "var(--severity-medium)", "⚡")
    } else {
        ("Very High", "var(--severity-high)", "⚠️")
    };
    let (bugs_level, bugs_color, bugs_icon) = if total_bugs < 0.5 {
        ("Low", "var(--success)", "✓")
    } else if total_bugs < 2.0 {
        ("Moderate", "var(--warning)", "!")
    } else if total_bugs < 5.0 {
        ("High", "var(--severity-medium)", "⚡")
    } else {
        ("Very High", "var(--severity-high)", "⚠️")
    };

    FormattedHalsteadMetrics {
        volume: format!("{avg_vol:.2}"),
        volume_level: vol_level.to_owned(),
        volume_color: vol_color.to_owned(),
        volume_icon: vol_icon.to_owned(),
        difficulty: format!("{avg_diff:.2}"),
        difficulty_level: diff_level.to_owned(),
        difficulty_color: diff_color.to_owned(),
        difficulty_icon: diff_icon.to_owned(),
        effort: format!("{avg_effort:.2}"),
        effort_level: eff_level.to_owned(),
        effort_color: eff_color.to_owned(),
        effort_icon: eff_icon.to_owned(),
        bugs: format!("{total_bugs:.2}"),
        bugs_level: bugs_level.to_owned(),
        bugs_color: bugs_color.to_owned(),
        bugs_icon: bugs_icon.to_owned(),
        time: format!("{:.2}", h_metrics.time / file_count),
        calculated_length: format!("{:.2}", h_metrics.calculated_length / file_count),
    }
}

pub(super) fn calculate_score(result: &AnalysisResult) -> OverallScore {
    let unused_count = result.unused_functions.len()
        + result.unused_classes.len()
        + result.unused_imports.len()
        + result.unused_variables.len()
        + result.unused_methods.len()
        + result.unused_parameters.len();

    let complexity_penalty = compute_complexity_penalty(result);
    let maintainability_penalty = compute_maintainability_penalty(result, unused_count);
    let reliability_penalty = compute_reliability_penalty(result);
    let security_penalty = compute_security_penalty(result);
    let style_penalty = compute_style_penalty(result);

    let complexity_score = (100.0 - complexity_penalty).max(0.0);
    let maintainability_score = (100.0 - maintainability_penalty).max(0.0);
    let security_score = (100.0 - security_penalty.min(100.0)).max(0.0);
    let reliability_score = (100.0 - reliability_penalty).max(0.0);
    let style_score = (100.0 - style_penalty).max(0.0);

    let weighted_score = (complexity_score * 0.35)
        + (maintainability_score * 0.25)
        + (security_score * 0.15)
        + (reliability_score * 0.15)
        + (style_score * 0.10);
    let total = weighted_score.round() as u8;

    let grade = match total {
        90..=100 => "A",
        75..=89 => "B",
        60..=74 => "C",
        40..=59 => "D",
        _ => "F",
    }
    .to_owned();

    let grade_color = |grade: &str| -> String {
        match grade {
            "A" => "#4ade80".to_owned(),
            "B" => "#a3e635".to_owned(),
            "C" => "#facc15".to_owned(),
            "D" => "#fb923c".to_owned(),
            _ => "#f87171".to_owned(),
        }
    };

    let complexity_grade = to_letter_grade(complexity_score as u8);
    let maintainability_grade = to_letter_grade(maintainability_score as u8);
    let security_grade = to_letter_grade(security_score as u8);
    let reliability_grade = to_letter_grade(reliability_score as u8);
    let style_grade = to_letter_grade(style_score as u8);

    OverallScore {
        total_score: total,
        grade,
        categories: vec![
            build_category(
                "Complexity",
                complexity_score as u8,
                0,
                complexity_grade,
                &grade_color,
            ),
            build_category(
                "Maintainability",
                maintainability_score as u8,
                unused_count,
                maintainability_grade,
                &grade_color,
            ),
            build_category(
                "Security",
                security_score as u8,
                result.secrets.len() + result.danger.len() + result.taint_findings.len(),
                security_grade,
                &grade_color,
            ),
            build_category(
                "Reliability",
                reliability_score as u8,
                0,
                reliability_grade,
                &grade_color,
            ),
            build_category("Style", style_score as u8, 0, style_grade, &grade_color),
        ],
    }
}

fn to_letter_grade(score: u8) -> &'static str {
    match score {
        90..=100 => "A",
        75..=89 => "B",
        60..=74 => "C",
        _ => "F",
    }
}

fn compute_complexity_penalty(result: &AnalysisResult) -> f64 {
    let file_penalty: f64 = result
        .file_metrics
        .iter()
        .filter(|file_metric| file_metric.sloc > 500)
        .map(|_| 2.0)
        .sum();
    let issue_penalty: f64 = result
        .quality
        .iter()
        .map(complexity_penalty_from_issue)
        .sum();
    (file_penalty + issue_penalty).min(25.0)
}

fn complexity_penalty_from_issue(issue: &Finding) -> f64 {
    let msg = issue.message.to_lowercase();
    let complex_penalty = if msg.contains("complex") {
        issue
            .message
            .split('=')
            .nth(1)
            .and_then(|s| s.trim_end_matches(')').parse::<f64>().ok())
            .map_or(5.0, |val| ((val - 10.0) * 2.0).max(0.0))
    } else {
        0.0
    };

    let nesting_penalty = if msg.contains("nested") || msg.contains("nesting") {
        5.0
    } else {
        0.0
    };

    let length_penalty = if msg.contains("too long") { 5.0 } else { 0.0 };
    complex_penalty + nesting_penalty + length_penalty
}

fn compute_maintainability_penalty(result: &AnalysisResult, unused_count: usize) -> f64 {
    let duplicate_clone_penalty: f64 = result
        .clones
        .iter()
        .filter(|clone| clone.is_duplicate)
        .map(|_| 3.0)
        .sum();
    ((unused_count as f64) * 2.0 + duplicate_clone_penalty).min(45.0)
}

fn compute_reliability_penalty(result: &AnalysisResult) -> f64 {
    let penalty: f64 = result
        .quality
        .iter()
        .filter(|issue| {
            let msg = issue.message.to_lowercase();
            msg.contains("error")
                || msg.contains("exception")
                || msg.contains("panic")
                || msg.contains("unwrap")
                || msg.contains("expect")
        })
        .map(|_| 5.0)
        .sum();
    penalty.min(15.0)
}

fn compute_security_penalty(result: &AnalysisResult) -> f64 {
    result.secrets.iter().map(|_| 30.0).sum::<f64>()
        + result.danger.iter().map(|_| 15.0).sum::<f64>()
        + result.taint_findings.iter().map(|_| 20.0).sum::<f64>()
}

fn compute_style_penalty(result: &AnalysisResult) -> f64 {
    let penalty: f64 = result
        .quality
        .iter()
        .filter(|issue| {
            let msg = issue.message.to_lowercase();
            !msg.contains("complex")
                && !msg.contains("nested")
                && !msg.contains("too long")
                && !msg.contains("error")
                && !msg.contains("exception")
                && !msg.contains("panic")
                && !msg.contains("unwrap")
        })
        .map(|_| 2.0)
        .sum();
    penalty.min(5.0)
}

fn build_category(
    name: &str,
    score: u8,
    issue_count: usize,
    grade: &str,
    grade_color: &impl Fn(&str) -> String,
) -> CategoryScore {
    CategoryScore {
        name: name.into(),
        score,
        issue_count,
        grade: grade.into(),
        color: grade_color(grade),
    }
}
