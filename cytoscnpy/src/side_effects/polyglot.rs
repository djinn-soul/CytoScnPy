//! Polyglot scanner for detecting `JavaScript` / `TypeScript` top-level side effects.

use std::path::Path;

use super::types::{SideEffectKind, SideEffectMatch};

const MAX_SNIPPET: usize = 120;

/// Detects top-level side effects in `JavaScript` and `TypeScript` files.
#[must_use]
pub fn detect_js_side_effects(source: &str, file: &Path) -> Vec<SideEffectMatch> {
    let mut matches = Vec::new();
    let mut brace_depth: i32 = 0;
    let mut in_block_comment = false;

    for (line_idx, line) in source.lines().enumerate() {
        let trimmed = line.trim();
        let line_num = line_idx + 1;

        if in_block_comment {
            if let Some(pos) = trimmed.find("*/") {
                in_block_comment = false;
                let remainder = trimmed[pos + 2..].trim();
                if remainder.is_empty() {
                    continue;
                }
            } else {
                continue;
            }
        }

        if trimmed.starts_with("/*") && !trimmed.contains("*/") {
            in_block_comment = true;
            continue;
        }

        if trimmed.starts_with("//") || trimmed.starts_with('*') {
            continue;
        }

        let event_pos = find_js_event_listener(trimmed);
        let call_pos = find_js_toplevel_call(trimmed);

        if let Some(pos) = event_pos {
            let net_before = net_brace_delta(&trimmed[..pos]);
            if brace_depth + net_before == 0 && !is_enclosed_in_decl(trimmed, pos) {
                matches.push(SideEffectMatch {
                    file: file.to_path_buf(),
                    line: line_num,
                    column: 1,
                    kind: SideEffectKind::JsEventListener,
                    snippet: trimmed.chars().take(MAX_SNIPPET).collect(),
                });
            }
        } else if let Some(pos) = call_pos {
            let net_before = net_brace_delta(&trimmed[..pos]);
            if brace_depth + net_before == 0 && !is_enclosed_in_decl(trimmed, pos) {
                matches.push(SideEffectMatch {
                    file: file.to_path_buf(),
                    line: line_num,
                    column: 1,
                    kind: SideEffectKind::JsTopLevelCall,
                    snippet: trimmed.chars().take(MAX_SNIPPET).collect(),
                });
            }
        }

        for ch in trimmed.chars() {
            match ch {
                '{' => brace_depth += 1,
                '}' => brace_depth = (brace_depth - 1).max(0),
                _ => {}
            }
        }
    }

    matches
}

fn net_brace_delta(s: &str) -> i32 {
    let mut delta = 0;
    for ch in s.chars() {
        match ch {
            '{' => delta += 1,
            '}' => delta -= 1,
            _ => {}
        }
    }
    delta
}

fn is_enclosed_in_decl(line: &str, pos: usize) -> bool {
    let prefix = &line[..pos];
    prefix.contains("function ")
        || prefix.contains("function(")
        || prefix.contains("=>")
        || prefix.starts_with("if ")
        || prefix.starts_with("if(")
}

fn find_js_event_listener(text: &str) -> Option<usize> {
    const PATTERNS: &[&str] = &["addEventListener(", ".on(", ".subscribe(", ".listen("];
    PATTERNS.iter().filter_map(|p| text.find(p)).min()
}

fn find_js_toplevel_call(text: &str) -> Option<usize> {
    const PATTERNS: &[&str] = &[
        "app.use(",
        "router.use(",
        "app.get(",
        "app.post(",
        "app.put(",
        "app.delete(",
        "fetch(",
        "axios.get(",
        "axios.post(",
        "http.createServer(",
    ];
    PATTERNS.iter().filter_map(|p| text.find(p)).min()
}
