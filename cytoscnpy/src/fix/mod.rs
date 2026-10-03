//! Shared fix module for auto-remediation.
//!
//! This module provides reusable code rewriting infrastructure
//! that can be used by multiple features:
//! - Clone detection (remove duplicate code)
//! - Dead code removal (remove unused functions/classes)
//! - Future: import cleanup, formatting fixes, etc.
//!
//! The core component is `ByteRangeRewriter`, which applies
//! edits using byte offsets to safely modify source code.

mod edit;
mod rewriter;

pub use edit::{Edit, EditBuilder, RewriteError};
pub use rewriter::ByteRangeRewriter;
