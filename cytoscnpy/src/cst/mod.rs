//! CST (Concrete Syntax Tree) module for precise source mapping.
//!
//! This module provides Tree-sitter based CST parsing for:
//! - Precise byte-range extraction (including decorators, comments)
//! - Comment preservation during `--fix` operations
//! - Safe code rewriting that respects formatting
//!
//! # Feature Gate
//!
//! This module is only available with the `cst` feature enabled:
//! ```bash
//! cargo build --features cst
//! ```
//!
//! # Design Principles
//!
//! - **Lazy parsing**: CST is parsed only when `--fix-*` flags are used
//! - **AST decides, CST cuts**: All semantic decisions use Ruff AST
//! - **Byte-range anchored**: Mapping uses byte offsets, not structural matching

#[cfg(feature = "cst")]
mod comments;
#[cfg(feature = "cst")]
mod mapper;
#[cfg(feature = "cst")]
mod parser;

#[cfg(feature = "cst")]
pub use comments::Comment;
#[cfg(feature = "cst")]
pub use mapper::AstCstMapper;
#[cfg(feature = "cst")]
pub use parser::{CstError, CstNode, CstParser, CstTree, Point};

/// Placeholder types for when CST feature is disabled
#[cfg(not(feature = "cst"))]
pub mod stub {
    /// Stub error for CST operations when feature disabled
    #[derive(Debug)]
    pub struct CstNotAvailable;

    impl std::fmt::Display for CstNotAvailable {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "CST parsing requires --features cst")
        }
    }

    impl std::error::Error for CstNotAvailable {}
}
