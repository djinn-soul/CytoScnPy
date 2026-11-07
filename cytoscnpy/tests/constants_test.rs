//! Comprehensive tests for pattern matching constants
//! Tests for constant detection and analysis.
#![allow(clippy::explicit_iter_loop)]
#![allow(clippy::uninlined_format_args)]

use cytoscnpy::constants::{
    get_auto_called, get_default_exclude_folders, get_framework_file_re, get_penalties,
    get_test_decor_re, get_test_file_re, get_test_import_re, get_test_method_pattern,
    get_unittest_lifecycle_methods,
};

#[path = "constants_test/cases_1.rs"]
mod cases_1;
#[path = "constants_test/cases_2.rs"]
mod cases_2;
