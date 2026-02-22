use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
/// Metrics calculated using Halstead's Complexity Measures.
pub struct HalsteadMetrics {
    /// N1: Total number of operators.
    pub h1: usize,
    /// N2: Total number of operands.
    pub h2: usize,
    /// n1: Number of distinct operators.
    pub n1: usize,
    /// n2: Number of distinct operands.
    pub n2: usize,
    /// Halstead Program Vocabulary (n1 + n2).
    pub vocabulary: f64,
    /// Halstead Program Length (N1 + N2).
    pub length: f64,
    /// Calculated Program Length (n1 * log2(n1) + n2 * log2(n2)).
    pub calculated_length: f64,
    /// Halstead Volume (Length * log2(Vocabulary)).
    pub volume: f64,
    /// Halstead Difficulty ((n1 / 2) * (N2 / n2)).
    pub difficulty: f64,
    /// Halstead Effort (Difficulty * Volume).
    pub effort: f64,
    /// Estimated implementation time (Effort / 18).
    pub time: f64,
    /// Estimated number of delivered bugs (Volume / 3000).
    pub bugs: f64,
}
