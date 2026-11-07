//! Tests for the fix module (`ByteRangeRewriter`).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use cytoscnpy::fix::{ByteRangeRewriter, Edit, EditBuilder, RewriteError};

fn lcg_next(seed: &mut u64) -> u64 {
    *seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
    *seed
}

#[path = "fix_test/cases_1.rs"]
mod cases_1;
