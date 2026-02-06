//! Pluggable secret recognizers.
//!
//! This module defines the `SecretRecognizer` trait and provides
//! implementations for different detection strategies.

mod ast;
mod custom;
mod entropy;
mod entropy_ast;
mod regex;
mod types;

pub use ast::AstRecognizer;
pub use custom::CustomRecognizer;
pub use entropy::EntropyRecognizer;
pub use regex::RegexRecognizer;
pub use types::{RawFinding, SecretRecognizer};

#[cfg(test)]
mod tests;
