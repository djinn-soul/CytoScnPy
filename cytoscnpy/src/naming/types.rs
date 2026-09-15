//! Data types and structures for naming style distribution and consistency analysis.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Detected naming convention style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NamingStyle {
    /// `snake_case`: e.g. `calculate_total`, `get_user_id`
    SnakeCase,
    /// `camelCase`: e.g. `calculateTotal`, `getUserId`
    CamelCase,
    /// `PascalCase`: e.g. `CalculateTotal`, `GetUserId`
    PascalCase,
    /// `SCREAMING_SNAKE_CASE`: e.g. `MAX_BUFFER_SIZE`, `DEFAULT_TIMEOUT`
    ScreamingSnakeCase,
    /// Mixed / non-standard casing: e.g. `calc_Total`, `get_userID`
    Mixed,
}

impl NamingStyle {
    /// Human-readable label for the naming style.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SnakeCase => "snake_case",
            Self::CamelCase => "camelCase",
            Self::PascalCase => "PascalCase",
            Self::ScreamingSnakeCase => "SCREAMING_SNAKE_CASE",
            Self::Mixed => "mixed",
        }
    }
}

impl std::fmt::Display for NamingStyle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// An identifier definition that does not conform to the dominant naming style.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NamingOutlier {
    /// Identifier name.
    pub name: String,
    /// File containing the definition.
    pub file: PathBuf,
    /// 1-indexed source line.
    pub line: usize,
    /// Detected style for this identifier.
    pub detected_style: NamingStyle,
    /// Dominant or expected style for this codebase.
    pub expected_style: NamingStyle,
}

/// Aggregate metrics and distribution of naming styles across the codebase.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NamingStats {
    /// Total identifiers analyzed (excluding dunders, empty, and anonymous).
    pub total_identifiers: usize,
    /// Count of `snake_case` identifiers.
    pub snake_case_count: usize,
    /// Count of `camelCase` identifiers.
    pub camel_case_count: usize,
    /// Count of `PascalCase` identifiers.
    pub pascal_case_count: usize,
    /// Count of `SCREAMING_SNAKE_CASE` identifiers.
    pub screaming_snake_count: usize,
    /// Count of mixed/irregular casing identifiers.
    pub mixed_count: usize,
    /// Count of Python dunder methods (e.g. `__init__`), exempted from style penalties.
    pub dunder_count: usize,
    /// Dominant style across all analyzed identifiers.
    pub dominant_style: NamingStyle,
    /// Ratio of dominant style identifiers to total analyzed identifiers (0.0 to 1.0).
    pub dominant_style_ratio: f64,
}

impl Default for NamingStats {
    fn default() -> Self {
        Self {
            total_identifiers: 0,
            snake_case_count: 0,
            camel_case_count: 0,
            pascal_case_count: 0,
            screaming_snake_count: 0,
            mixed_count: 0,
            dunder_count: 0,
            dominant_style: NamingStyle::SnakeCase,
            dominant_style_ratio: 1.0,
        }
    }
}

impl NamingStats {
    /// Style consistency score expressed as a percentage (0.0% - 100.0%).
    #[must_use]
    pub fn consistency_score(&self) -> f64 {
        self.dominant_style_ratio * 100.0
    }

    /// Calculates the sum of all classified identifiers (excluding dunders).
    #[must_use]
    pub const fn classified_total(&self) -> usize {
        self.snake_case_count
            + self.camel_case_count
            + self.pascal_case_count
            + self.screaming_snake_count
            + self.mixed_count
    }

    /// Returns the count for a specific naming style.
    #[must_use]
    pub const fn count_for_style(&self, style: NamingStyle) -> usize {
        match style {
            NamingStyle::SnakeCase => self.snake_case_count,
            NamingStyle::CamelCase => self.camel_case_count,
            NamingStyle::PascalCase => self.pascal_case_count,
            NamingStyle::ScreamingSnakeCase => self.screaming_snake_count,
            NamingStyle::Mixed => self.mixed_count,
        }
    }

    /// Returns the percentage of a specific style relative to classified total.
    ///
    /// In standard scans, `total_identifiers == classified_total()`.
    #[must_use]
    pub fn percentage_for_style(&self, style: NamingStyle) -> f64 {
        let total = self.classified_total();
        if total == 0 {
            return 0.0;
        }
        (self.count_for_style(style) as f64 / total as f64) * 100.0
    }
}

/// Normalizes a consistency threshold specified either as a ratio (0.0 - 1.0) or a percentage (0.0 - 100.0).
#[must_use]
pub fn normalize_consistency_threshold(threshold: f64) -> f64 {
    if threshold > 1.0 {
        threshold / 100.0
    } else {
        threshold
    }
}

/// Comprehensive result of naming style distribution and consistency analysis.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NamingDistributionResult {
    /// Aggregate statistics and ratios.
    pub stats: NamingStats,
    /// List of outlier identifiers deviating from the dominant style.
    pub outliers: Vec<NamingOutlier>,
}

impl NamingDistributionResult {
    /// Checks whether naming consistency meets or exceeds a target ratio (0.0 to 1.0 or 0 to 100).
    #[must_use]
    pub fn is_consistent(&self, min_ratio: f64) -> bool {
        let normalized = normalize_consistency_threshold(min_ratio);
        self.stats.dominant_style_ratio >= normalized
    }
}
