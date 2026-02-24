//! Clone detection module for CytoScnPy.
//!
//! This module provides code clone detection with Type-1/2/3 support:
//! - Type-1: Exact clones (whitespace/comment differences)
//! - Type-2: Renamed identifiers/literals
//! - Type-3: Near-miss clones (statements added/removed)
//!
//! For code rewriting, use the shared `crate::fix` module.

mod confidence;
mod config;
mod detector;
mod errors;
mod findings;
mod hasher;
mod normalizer;
mod parser;
mod similarity;
mod types;

// Re-exports
pub use confidence::{ConfidenceScorer, FixConfidence, FixContext, FixDecision};
pub use config::CloneConfig;
pub use detector::{CloneDetectionResult, CloneDetector};
pub use errors::CloneError;
pub use findings::{CloneFinding, CloneRelation};
pub use normalizer::Normalizer;
pub use parser::{extract_subtrees, Subtree, SubtreeNode, SubtreeType};
pub use similarity::TreeSimilarity;
pub use types::{CloneGroup, CloneInstance, ClonePair, CloneSummary, CloneType, NodeKind};
// Re-export from shared fix module for convenience
pub use crate::fix::ByteRangeRewriter;
