use serde::{Deserialize, Serialize};
use std::fmt;

/// Qualitative verdict band based on the 0-100 Slop Index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Verdict {
    /// 0 - 20: Excellent architectural discipline and minimal LLM friction.
    Clean,
    /// 21 - 40: Normal codebase with minor slop within reasonable boundaries.
    Acceptable,
    /// 41 - 60: Notable architectural friction, high complexity, or poor boundaries.
    Messy,
    /// 61 - 80: High architectural debt, significant hazards, and LLM confusion risk.
    Sloppy,
    /// 81 - 100: Severe structural degradation, high failure likelihood for agents.
    Disaster,
}

impl Verdict {
    /// Map a numerical slop index (0-100) to its verdict band.
    #[must_use]
    pub fn from_index(index: u32) -> Self {
        match index {
            0..=20 => Self::Clean,
            21..=40 => Self::Acceptable,
            41..=60 => Self::Messy,
            61..=80 => Self::Sloppy,
            _ => Self::Disaster,
        }
    }

    /// Whether the verdict is considered passing in standard CI mode.
    #[must_use]
    pub fn is_passing(self) -> bool {
        matches!(self, Self::Clean | Self::Acceptable)
    }
}

impl fmt::Display for Verdict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Clean => write!(f, "CLEAN"),
            Self::Acceptable => write!(f, "ACCEPTABLE"),
            Self::Messy => write!(f, "MESSY"),
            Self::Sloppy => write!(f, "SLOPPY"),
            Self::Disaster => write!(f, "DISASTER"),
        }
    }
}

/// Score and evidence for one of the 10 `DeSlopify` dimensions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DimensionScore {
    /// Dimension name (e.g. "Architecture clarity", "Setup reliability").
    pub name: String,
    /// Dimension weight (weights across all 10 dimensions sum to 100).
    pub weight: u32,
    /// Slop rating from 0 (pristine / no slop) to 5 (maximum slop).
    pub rating: u32,
    /// Weighted score contribution: weight * (rating / 5.0).
    pub raw_contribution: f64,
    /// Human-readable explanation and empirical findings for this rating.
    pub evidence: String,
}

/// Complete result of the weighted Slop Index calculation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScoreResult {
    /// Final weighted Slop Index from 0 to 100 (after size multiplier).
    pub slop_index: u32,
    /// Raw unadjusted score (sum of all dimension contributions, 0.0 - 100.0).
    pub raw_score: f64,
    /// Scale factor [0.0, 1.0] scaling score to effective codebase token capacity.
    pub size_multiplier: f64,
    /// Categorical verdict band (CLEAN, ACCEPTABLE, MESSY, SLOPPY, DISASTER).
    pub verdict: Verdict,
    /// Breakdown of all 10 scored dimensions.
    pub dimensions: Vec<DimensionScore>,
    /// Prioritized remediation recommendations ranked by estimated score reduction.
    pub recommendations: Vec<super::recommendations::Recommendation>,
    /// Summary of repository structure, languages, test files, and configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<super::summary::RepoSummary>,
    /// Whether the score satisfies all configured gates and thresholds.
    pub passed_gate: bool,
    /// Failure reason if gate failed.
    pub failure_reason: Option<String>,
}

impl ScoreResult {
    /// Check whether the score is completely clean.
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.slop_index == 0 && self.dimensions.iter().all(|d| d.rating == 0)
    }
}

/// Configuration options for the scoring engine.
#[derive(Debug, Clone)]
pub struct ScoringOptions {
    /// Maximum allowed Slop Index before gate failure.
    pub max_score: Option<u32>,
    /// Fail if any individual dimension rating is > 0.
    pub fail_on_any: bool,
    /// CI mode: requires passing verdict (Clean or Acceptable) and `max_score` gate.
    pub ci: bool,
    /// Usable context budget in tokens (default: 176,000).
    pub context_budget: usize,
    /// Disable Git history analysis for size multiplier.
    pub no_git: bool,
    /// Maximum Git lookback window in months.
    pub git_months: Option<u32>,
}

impl Default for ScoringOptions {
    fn default() -> Self {
        Self {
            max_score: None,
            fail_on_any: false,
            ci: false,
            context_budget: 176_000,
            no_git: false,
            git_months: None,
        }
    }
}
