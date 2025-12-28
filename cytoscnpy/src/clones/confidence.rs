//! Confidence scoring system for clone detection.
//!
//! This module provides a reusable confidence scoring system
//! that can be used for deciding auto-fix vs manual review.

use crate::clones::types::ClonePair;

/// Decision based on confidence score
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixDecision {
    /// Score >= `auto_fix_threshold`: Apply fix automatically
    AutoFix,
    /// `suggest_threshold` <= score < `auto_fix_threshold`: Show as suggestion
    Suggest,
    /// Score < `suggest_threshold`: Do not report
    Suppress,
}

/// A single factor contributing to confidence
#[derive(Debug, Clone)]
pub struct ConfidenceFactor {
    /// Factor name
    pub name: &'static str,
    /// Score adjustment (-100 to +100)
    pub adjustment: i8,
    /// Human-readable reason
    pub reason: String,
}

impl ConfidenceFactor {
    /// Create a new confidence factor
    #[must_use]
    pub fn new(name: &'static str, adjustment: i8, reason: impl Into<String>) -> Self {
        Self {
            name,
            adjustment,
            reason: reason.into(),
        }
    }
}

/// Confidence score with justification
#[derive(Debug, Clone)]
pub struct FixConfidence {
    /// Final score (0-100)
    pub score: u8,
    /// Factors that contributed to the score
    pub factors: Vec<ConfidenceFactor>,
    /// Resulting decision
    pub decision: FixDecision,
}

/// Context for confidence scoring
#[derive(Debug, Clone, Default)]
pub struct FixContext {
    /// Is either file a test file?
    pub is_test_file: bool,
    /// Are both instances in the same file?
    pub same_file: bool,
    /// Does the canonical version have a docstring?
    pub canonical_has_docstring: bool,
    /// Do control flow structures differ?
    pub control_flow_differs: bool,
    /// Has structural match been verified?
    pub structural_match_verified: bool,
    /// Is the fix idempotent?
    pub is_idempotent: bool,
    // ── CST-derived context (populated when --fix-* enabled) ──
    /// Are there interleaved comments within the definition body?
    pub has_interleaved_comments: bool,
    /// Do decorators differ between clones?
    pub decorators_differ: bool,
    /// Is the definition deeply nested (>2 levels)?
    pub deeply_nested: bool,
    // ── CFG-derived context (populated when cfg_validation enabled) ──
    /// Has CFG behavioral validation passed?
    /// When true, the clone pair has been verified to have similar
    /// control flow structure (loops, branches, etc.)
    pub cfg_validated: bool,
}

/// Confidence scorer with configurable thresholds
#[derive(Debug, Clone)]
pub struct ConfidenceScorer {
    /// Threshold for auto-fix (default: 90)
    pub auto_fix_threshold: u8,
    /// Threshold for suggestions (default: 60)
    pub suggest_threshold: u8,
}

impl Default for ConfidenceScorer {
    fn default() -> Self {
        Self {
            auto_fix_threshold: 90,
            suggest_threshold: 60,
        }
    }
}

impl ConfidenceScorer {
    /// Create a new scorer with custom thresholds
    #[must_use]
    pub const fn new(auto_fix_threshold: u8, suggest_threshold: u8) -> Self {
        Self {
            auto_fix_threshold,
            suggest_threshold,
        }
    }

    /// Calculate confidence for a clone pair with context
    #[must_use]
    pub fn score(&self, pair: &ClonePair, context: &FixContext) -> FixConfidence {
        let mut score: i16 = 50; // Base score
        let mut factors = Vec::new();

        // ══════════════════════════════════════════════════════════════
        // SIMILARITY-BASED FACTORS
        // ══════════════════════════════════════════════════════════════

        if pair.similarity >= 0.95 {
            score += 30;
            factors.push(ConfidenceFactor::new("similarity", 30, "≥ 95%"));
        } else if pair.similarity >= 0.90 {
            score += 20;
            factors.push(ConfidenceFactor::new("similarity", 20, "≥ 90%"));
        } else if pair.similarity >= 0.80 {
            score += 10;
            factors.push(ConfidenceFactor::new("similarity", 10, "≥ 80%"));
        } else if pair.similarity < 0.70 {
            score -= 30;
            factors.push(ConfidenceFactor::new("similarity", -30, "< 70%"));
        }

        // Clone type confidence
        let type_bonus = pair.clone_type.confidence_bonus();
        score += i16::from(type_bonus);
        factors.push(ConfidenceFactor::new(
            "clone_type",
            type_bonus,
            format!("{:?}", pair.clone_type),
        ));

        // ══════════════════════════════════════════════════════════════
        // CONTEXT-BASED FACTORS
        // ══════════════════════════════════════════════════════════════

        if context.is_test_file {
            score -= 20;
            factors.push(ConfidenceFactor::new("test_file", -20, "test file"));
        }

        if context.same_file {
            score += 10;
            factors.push(ConfidenceFactor::new("same_file", 10, "same file"));
        }

        if context.canonical_has_docstring {
            score += 10;
            factors.push(ConfidenceFactor::new("docstring", 10, "has docstring"));
        }

        // ══════════════════════════════════════════════════════════════
        // STRUCTURAL FACTORS
        // ══════════════════════════════════════════════════════════════

        if pair.edit_distance <= 2 {
            score += 15;
            factors.push(ConfidenceFactor::new("edit_distance", 15, "≤ 2"));
        } else if pair.edit_distance >= 10 {
            score -= 20;
            factors.push(ConfidenceFactor::new("edit_distance", -20, "≥ 10"));
        }

        if context.control_flow_differs {
            score -= 40;
            factors.push(ConfidenceFactor::new(
                "control_flow",
                -40,
                "control flow differs",
            ));
        }

        // ══════════════════════════════════════════════════════════════
        // SAFETY FACTORS
        // ══════════════════════════════════════════════════════════════

        if context.structural_match_verified {
            score += 20;
            factors.push(ConfidenceFactor::new("verified", 20, "match verified"));
        }

        if context.is_idempotent {
            score += 10;
            factors.push(ConfidenceFactor::new("idempotent", 10, "idempotent fix"));
        }

        // ══════════════════════════════════════════════════════════════
        // CST-DERIVED FACTORS (when --fix-* enabled)
        // ══════════════════════════════════════════════════════════════

        if context.has_interleaved_comments {
            score -= 15;
            factors.push(ConfidenceFactor::new(
                "interleaved_comments",
                -15,
                "comments interleaved in body",
            ));
        }

        if context.decorators_differ {
            score -= 20;
            factors.push(ConfidenceFactor::new(
                "decorators_differ",
                -20,
                "decorators differ between clones",
            ));
        }

        if context.deeply_nested {
            score -= 10;
            factors.push(ConfidenceFactor::new(
                "deeply_nested",
                -10,
                "definition nested >2 levels",
            ));
        }

        // ══════════════════════════════════════════════════════════════
        // CFG-DERIVED FACTORS (when cfg_validation enabled)
        // ══════════════════════════════════════════════════════════════

        if context.cfg_validated {
            score += 15;
            factors.push(ConfidenceFactor::new(
                "cfg_validated",
                15,
                "CFG behavioral match verified",
            ));
        }

        // Clamp to 0-100
        #[allow(clippy::cast_sign_loss)]
        let final_score = score.clamp(0, 100) as u8;

        let decision = if final_score >= self.auto_fix_threshold {
            FixDecision::AutoFix
        } else if final_score >= self.suggest_threshold {
            FixDecision::Suggest
        } else {
            FixDecision::Suppress
        };

        FixConfidence {
            score: final_score,
            factors,
            decision,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clones::types::{CloneInstance, CloneType};
    use std::path::PathBuf;

    fn make_pair(similarity: f64, clone_type: CloneType, edit_distance: usize) -> ClonePair {
        use crate::clones::types::NodeKind;

        ClonePair {
            instance_a: CloneInstance {
                file: PathBuf::from("a.py"),
                start_line: 1,
                end_line: 10,
                start_byte: 0,
                end_byte: 100,
                normalized_hash: 0,
                name: None,
                node_kind: NodeKind::Function,
            },
            instance_b: CloneInstance {
                file: PathBuf::from("b.py"),
                start_line: 1,
                end_line: 10,
                start_byte: 0,
                end_byte: 100,
                normalized_hash: 0,
                name: None,
                node_kind: NodeKind::Function,
            },
            similarity,
            clone_type,
            edit_distance,
        }
    }

    #[test]
    fn test_high_confidence_auto_fix() {
        let scorer = ConfidenceScorer::default();
        let pair = make_pair(0.98, CloneType::Type1, 0);
        let context = FixContext {
            structural_match_verified: true,
            is_idempotent: true,
            ..Default::default()
        };

        let result = scorer.score(&pair, &context);
        assert_eq!(result.decision, FixDecision::AutoFix);
        assert!(result.score >= 90);
    }

    #[test]
    fn test_low_confidence_suppress() {
        let scorer = ConfidenceScorer::default();
        let pair = make_pair(0.65, CloneType::Type3, 15);
        let context = FixContext {
            is_test_file: true,
            control_flow_differs: true,
            ..Default::default()
        };

        let result = scorer.score(&pair, &context);
        assert_eq!(result.decision, FixDecision::Suppress);
    }
}
