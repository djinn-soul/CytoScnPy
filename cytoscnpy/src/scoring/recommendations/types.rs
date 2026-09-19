//! Data structures for prioritized codebase remediation recommendations.

use serde::{Deserialize, Serialize};

/// Implementation effort level for a remediation recommendation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Effort {
    /// Trivial or mechanical change (e.g. adding tool config, renaming colliding name).
    Low,
    /// Moderate refactoring (e.g. encapsulating globals, breaking simple import cycle).
    Medium,
    /// Significant architectural restructuring (e.g. splitting god module, extracting package).
    High,
}

impl std::fmt::Display for Effort {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Low => write!(f, "Low"),
            Self::Medium => write!(f, "Medium"),
            Self::High => write!(f, "High"),
        }
    }
}

/// A concrete, prioritized remediation recommendation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Recommendation {
    /// Unique identifier for the recommendation rule.
    pub id: String,
    /// Short human-readable title.
    pub title: String,
    /// The `DeSlopify` scoring dimension this recommendation targets.
    pub dimension: String,
    /// Estimated Slop Index point reduction if addressed (0-100).
    pub estimated_reduction: u32,
    /// Target dimension rating after remediation (0-5).
    pub target_rating: u32,
    /// Estimated implementation effort.
    pub effort: Effort,
    /// Problem explanation and rationale.
    pub description: String,
    /// Step-by-step instructions for an engineer or coding agent.
    pub action_steps: Vec<String>,
    /// Relative or canonical file paths directly affected.
    pub affected_files: Vec<String>,
}
