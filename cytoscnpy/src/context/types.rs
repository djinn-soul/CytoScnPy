use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::PathBuf;

/// Risk level for high-churn, complex files.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum HotspotRiskLevel {
    /// Low churn and manageable complexity.
    Low,
    /// Moderate churn or complexity.
    Medium,
    /// High churn and high complexity.
    High,
    /// Extremely high churn coupled with high complexity.
    Critical,
}

impl fmt::Display for HotspotRiskLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Low => write!(f, "Low"),
            Self::Medium => write!(f, "Medium"),
            Self::High => write!(f, "High"),
            Self::Critical => write!(f, "Critical"),
        }
    }
}

/// A file identified with high commit churn.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HotFile {
    /// Absolute or relative file path.
    pub path: PathBuf,
    /// Human-friendly display path.
    pub display_path: String,
    /// Number of commits modifying this file in lookback window.
    pub commit_count: usize,
    /// Line count of the file.
    pub lines: usize,
    /// Size of the file in bytes.
    pub bytes: u64,
}

/// A file combining high commit churn and high cyclomatic complexity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HotspotFile {
    /// Path to the hotspot file.
    pub path: PathBuf,
    /// Human-friendly relative display path.
    pub display_path: String,
    /// Number of commits modifying this file.
    pub commit_count: usize,
    /// Total cyclomatic complexity score.
    pub cyclomatic_complexity: usize,
    /// Combined risk score (churn * complexity).
    pub risk_score: usize,
    /// Categorized risk level.
    pub risk_level: HotspotRiskLevel,
}

/// Activity statistics derived from Git history over a lookback window.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitActivity {
    /// Whether target path is inside a Git repository.
    pub is_git_repo: bool,
    /// Number of files modified in the lookback window.
    pub active_files: usize,
    /// Total lines in active files.
    pub active_lines: usize,
    /// Total bytes in active files.
    pub active_bytes: u64,
    /// Number of files untouched in the lookback window.
    pub frozen_files: usize,
    /// Total lines in frozen files.
    pub frozen_lines: usize,
    /// Total bytes in frozen files.
    pub frozen_bytes: u64,
    /// Total commits in the repository within lookback window.
    pub total_commits: usize,
    /// Lookback window in days.
    pub window_days: u32,
    /// Formatted lookback label (e.g. "30 days", "2 weeks").
    pub window_label: String,
    /// Top most churned files in the window.
    pub hot_files: Vec<HotFile>,
}

impl Default for GitActivity {
    fn default() -> Self {
        Self {
            is_git_repo: false,
            active_files: 0,
            active_lines: 0,
            active_bytes: 0,
            frozen_files: 0,
            frozen_lines: 0,
            frozen_bytes: 0,
            total_commits: 0,
            window_days: 30,
            window_label: "30 days".to_owned(),
            hot_files: Vec::new(),
        }
    }
}

/// Status verdict on LLM context headroom.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContextStatusVerdict {
    /// Less than 25% of context window used.
    Generous,
    /// Between 25% and 50% of context window used.
    Healthy,
    /// Between 50% and 75% of context window used.
    Moderate,
    /// Between 75% and 90% of context window used.
    Constrained,
    /// Exceeds 90% of usable context capacity.
    Exhausted,
}

impl fmt::Display for ContextStatusVerdict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Generous => write!(f, "Generous"),
            Self::Healthy => write!(f, "Healthy"),
            Self::Moderate => write!(f, "Moderate"),
            Self::Constrained => write!(f, "Constrained"),
            Self::Exhausted => write!(f, "Exhausted"),
        }
    }
}

/// Estimated token costs and context headroom for an LLM agent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TokenBudget {
    /// Estimated tokens spent discovering relevant files.
    pub file_discovery_tokens: usize,
    /// Estimated tokens required to read necessary files.
    pub code_reading_tokens: usize,
    /// Estimated tokens to follow import and dependency chains.
    pub dependency_tracing_tokens: usize,
    /// Estimated cognitive comprehension overhead tokens.
    pub comprehension_tokens: usize,
    /// Total navigation tokens (discovery + reading + tracing + comprehension).
    pub total_navigation_tokens: usize,
    /// Total usable context window capacity in tokens.
    pub usable_context: usize,
    /// Percentage of usable context consumed by navigation.
    pub navigation_pct: f64,
    /// Remaining headroom in tokens for prompt instructions and code changes.
    pub remaining_tokens: usize,
    /// Qualitative status verdict on context pressure.
    pub status_verdict: ContextStatusVerdict,
}

/// Configuration options for context and git analysis.
#[derive(Debug, Clone)]
pub struct ContextConfig {
    /// Lookback window in months (optional override).
    pub git_months: Option<u32>,
    /// Configured context budget in tokens (default: 176,000).
    pub context_budget: usize,
    /// Disable Git history inspection.
    pub no_git: bool,
    /// Only display hotspot files.
    pub hotspots_only: bool,
    /// Exit with error code 1 if severe hotspots are present.
    pub fail_on_hotspots: bool,
    /// Verbose output logging.
    pub verbose: bool,
}

impl Default for ContextConfig {
    fn default() -> Self {
        Self {
            git_months: None,
            context_budget: 176_000,
            no_git: false,
            hotspots_only: false,
            fail_on_hotspots: false,
            verbose: false,
        }
    }
}

/// Complete result of context, churn, and token budget analysis.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContextAnalysisResult {
    /// Total scanned source files.
    pub total_files: usize,
    /// Total lines across scanned files.
    pub total_lines: usize,
    /// Total byte size of scanned files.
    pub total_bytes: u64,
    /// Git churn and active/frozen breakdown.
    pub git_activity: GitActivity,
    /// Discovered churn-complexity hotspots.
    pub hotspots: Vec<HotspotFile>,
    /// Estimated token budget breakdown.
    pub token_budget: TokenBudget,
}
