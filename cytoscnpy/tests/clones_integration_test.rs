#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::str_to_string,
    clippy::missing_docs_in_private_items,
    missing_docs
)]

use cytoscnpy::clones::{
    extract_subtrees, CloneConfig, CloneDetector, CloneType, NodeKind, SubtreeNode,
};
use std::path::PathBuf;

#[path = "clones_integration_test/cases_1.rs"]
mod cases_1;
#[path = "clones_integration_test/cases_2.rs"]
mod cases_2;
