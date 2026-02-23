#![allow(missing_docs)]
#![allow(
    clippy::wildcard_imports,
    clippy::elidable_lifetime_names,
    clippy::semicolon_if_nothing_returned,
    clippy::needless_pass_by_value
)]

use crate::constants::MAX_RECURSION_DEPTH;
use crate::constants::PYTEST_HOOKS;
use crate::utils::LineIndex;
use compact_str::CompactString;
use regex::Regex;
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::Ranged;
use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use smallvec::SmallVec;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::LazyLock;

static EVAL_IDENTIFIER_RE: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"\b[a-zA-Z_]\w*\b").ok());

mod constructor;
mod definition;
mod expr;
mod expr_traversal;
mod function_def;
mod function_def_meta;
mod function_def_params;
mod scope_ops;
mod state;
mod stmt;
mod stmt_assignments;
mod stmt_control_flow;
mod stmt_defs;
mod stmt_imports;
mod stmt_misc;
mod targets;
mod types;

pub use state::CytoScnPyVisitor;
pub use types::{Definition, DefinitionInfo, Scope, ScopeType, UnusedCategory};
