//! Polyglot scanners for detecting mutable global state in Rust and JavaScript/TypeScript.

use std::path::Path;

use super::types::{GlobalKind, GlobalMatch};

/// Detects `static mut` declarations in Rust source code.
#[must_use]
pub fn detect_rust_globals(source: &str, file: &Path) -> Vec<GlobalMatch> {
    let mut matches = Vec::new();

    for (idx, line) in source.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with('*') {
            continue;
        }

        let after_pub = trimmed.strip_prefix("pub ").unwrap_or(trimmed).trim_start();
        if let Some(rest) = after_pub.strip_prefix("static mut ") {
            let name = rest
                .trim_start()
                .split(|c: char| !c.is_alphanumeric() && c != '_')
                .next()
                .unwrap_or("");

            if !name.is_empty() {
                let col = line.find("static").unwrap_or(0) + 1;
                matches.push(GlobalMatch {
                    file: file.to_path_buf(),
                    line: idx + 1,
                    column: col,
                    name: name.to_owned(),
                    kind: GlobalKind::RustStaticMut,
                    snippet: trimmed.to_owned(),
                });
            }
        }
    }

    matches
}

/// Detects top-level mutable `var` and mutable collection `let` declarations in JavaScript/TypeScript.
#[must_use]
pub fn detect_js_globals(source: &str, file: &Path) -> Vec<GlobalMatch> {
    let mut matches = Vec::new();

    for (idx, line) in source.lines().enumerate() {
        if line.starts_with(' ') || line.starts_with('\t') {
            continue;
        }
        let trimmed = line.trim();
        if trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with('*') {
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("var ") {
            if let Some((lhs, _)) = rest.split_once('=') {
                let name = lhs.trim();
                if !name.is_empty() {
                    matches.push(GlobalMatch {
                        file: file.to_path_buf(),
                        line: idx + 1,
                        column: 1,
                        name: name.to_owned(),
                        kind: GlobalKind::JsTopLevelMutable,
                        snippet: trimmed.to_owned(),
                    });
                }
            }
        } else if let Some(rest) = trimmed.strip_prefix("let ") {
            if let Some((lhs, rhs)) = rest.split_once('=') {
                let name = lhs.trim();
                let rhs_trimmed = rhs.trim_start();
                if rhs_trimmed.starts_with('[') || rhs_trimmed.starts_with('{') {
                    matches.push(GlobalMatch {
                        file: file.to_path_buf(),
                        line: idx + 1,
                        column: 1,
                        name: name.to_owned(),
                        kind: GlobalKind::JsTopLevelMutable,
                        snippet: trimmed.to_owned(),
                    });
                }
            }
        }
    }

    matches
}
