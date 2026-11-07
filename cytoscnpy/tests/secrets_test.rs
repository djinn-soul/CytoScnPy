//! Tests for secret scanning rules.
//!
//! Verifies entropy calculation, pattern matching (AWS, generic secrets), and configuration handling.
#![allow(
    clippy::float_cmp,
    clippy::uninlined_format_args,
    clippy::str_to_string,
    clippy::field_reassign_with_default
)]

use cytoscnpy::config::{CustomSecretPattern, SecretsConfig};

use cytoscnpy::rules::secrets::{
    calculate_entropy, is_high_entropy, scan_secrets, scan_secrets_compat,
};
use std::path::PathBuf;

#[path = "secrets_test/cases_1.rs"]
mod cases_1;
#[path = "secrets_test/cases_2.rs"]
mod cases_2;
