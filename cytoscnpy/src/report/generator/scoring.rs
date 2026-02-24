#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::too_many_lines
)]

use crate::analyzer::AnalysisResult;
use crate::report::templates::{CategoryScore, FormattedHalsteadMetrics, OverallScore};

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
    let mut complexity_penalty: f64 = 0.0;

    for file_metric in &result.file_metrics {
        if file_metric.sloc > 500 {
            complexity_penalty += 2.0;
        }
    }

    for issue in &result.quality {
        if issue.message.to_lowercase().contains("complex") {
            if let Some(val) = issue
                .message
                .split('=')
                .nth(1)
                .and_then(|s| s.trim_end_matches(')').parse::<f64>().ok())
            {
                if val > 10.0 {
                    complexity_penalty += (val - 10.0) * 2.0;
                }
            } else {
                complexity_penalty += 5.0;
            }
        }
        if issue.message.to_lowercase().contains("nested")
            || issue.message.to_lowercase().contains("nesting")
        {
            complexity_penalty += 5.0;
        }
        if issue.message.to_lowercase().contains("too long") {
            complexity_penalty += 5.0;
        }
    }
    complexity_penalty = complexity_penalty.min(25.0);

    let mut maintainability_penalty: f64 = 0.0;
    let unused_count = result.unused_functions.len()
        + result.unused_classes.len()
        + result.unused_imports.len()
        + result.unused_variables.len()
        + result.unused_methods.len()
        + result.unused_parameters.len();

    maintainability_penalty += (unused_count as f64) * 2.0;

    for clone in &result.clones {
        if clone.is_duplicate {
            maintainability_penalty += 3.0;
        }
    }
    maintainability_penalty = maintainability_penalty.min(45.0);

    let mut reliability_penalty: f64 = 0.0;
    for issue in &result.quality {
        let msg = issue.message.to_lowercase();
        if msg.contains("error")
            || msg.contains("exception")
            || msg.contains("panic")
            || msg.contains("unwrap")
            || msg.contains("expect")
        {
            reliability_penalty += 5.0;
        }
    }
    reliability_penalty = reliability_penalty.min(15.0);

    let mut security_penalty: f64 = 0.0;
    for _ in &result.secrets {
        security_penalty += 30.0;
    }
    for _ in &result.danger {
        security_penalty += 15.0;
    }
    for _ in &result.taint_findings {
        security_penalty += 20.0;
    }

    let mut style_penalty: f64 = 0.0;
    for issue in &result.quality {
        let msg = issue.message.to_lowercase();
        if !msg.contains("complex")
            && !msg.contains("nested")
            && !msg.contains("too long")
            && !msg.contains("error")
            && !msg.contains("exception")
            && !msg.contains("panic")
            && !msg.contains("unwrap")
        {
            style_penalty += 2.0;
        }
    }
    style_penalty = style_penalty.min(5.0);

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
            CategoryScore {
                name: "Complexity".into(),
                score: complexity_score as u8,
                issue_count: 0,
                grade: complexity_grade.into(),
                color: grade_color(complexity_grade),
            },
            CategoryScore {
                name: "Maintainability".into(),
                score: maintainability_score as u8,
                issue_count: unused_count,
                grade: maintainability_grade.into(),
                color: grade_color(maintainability_grade),
            },
            CategoryScore {
                name: "Security".into(),
                score: security_score as u8,
                issue_count: result.secrets.len()
                    + result.danger.len()
                    + result.taint_findings.len(),
                grade: security_grade.into(),
                color: grade_color(security_grade),
            },
            CategoryScore {
                name: "Reliability".into(),
                score: reliability_score as u8,
                issue_count: 0,
                grade: reliability_grade.into(),
                color: grade_color(reliability_grade),
            },
            CategoryScore {
                name: "Style".into(),
                score: style_score as u8,
                issue_count: 0,
                grade: style_grade.into(),
                color: grade_color(style_grade),
            },
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
