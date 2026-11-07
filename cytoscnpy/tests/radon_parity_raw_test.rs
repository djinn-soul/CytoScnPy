//! Comprehensive Radon raw metrics parity tests.
//! Ported from: `radon/tests/test_raw.py`
#![allow(clippy::needless_raw_string_hashes)]

use cytoscnpy::raw_metrics::analyze_raw;

#[path = "radon_parity_raw_test/cases_1.rs"]
mod cases_1;
#[path = "radon_parity_raw_test/cases_2.rs"]
mod cases_2;
#[path = "radon_parity_raw_test/cases_3.rs"]
mod cases_3;
