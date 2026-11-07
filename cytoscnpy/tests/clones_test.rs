//! Tests for the clone detection module.
//!
//! This module contains unit tests for clone type classification, data structures,
//! serialization, parser functionality, and clone detection algorithms.

#![allow(clippy::expect_used)]

use cytoscnpy::clones::{
    extract_subtrees, CloneConfig, CloneDetector, CloneFinding, CloneGroup, CloneInstance,
    ClonePair, CloneRelation, CloneSummary, CloneType, ConfidenceScorer, FixContext, NodeKind,
    SubtreeType,
};
use std::path::PathBuf;

#[path = "clones_test/cases_1.rs"]
mod cases_1;
#[path = "clones_test/cases_2.rs"]
mod cases_2;
#[path = "clones_test/cases_3.rs"]
mod cases_3;
