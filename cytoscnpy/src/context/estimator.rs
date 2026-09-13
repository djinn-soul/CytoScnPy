#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]

use super::types::{ContextStatusVerdict, TokenBudget};

/// Average characters per token for Python source code.
pub const PYTHON_CHARS_PER_TOKEN: f64 = 3.5;

/// Estimates token count for a text snippet using language-aware heuristics.
#[must_use]
pub fn estimate_tokens_for_content(content: &str) -> usize {
    if content.is_empty() {
        return 0;
    }

    let mut word_chars: usize = 0;
    let mut symbol_count: usize = 0;

    for ch in content.chars() {
        if ch.is_alphanumeric() || ch == '_' {
            word_chars += 1;
        } else if !ch.is_whitespace() {
            symbol_count += 1;
        }
    }

    let word_tokens = (word_chars as f64 / PYTHON_CHARS_PER_TOKEN).ceil() as usize;
    word_tokens + symbol_count
}

/// Estimates tokens spent during repository and file discovery.
#[must_use]
pub fn estimate_discovery_tokens(total_files: usize, filename_collisions: usize) -> usize {
    let search_queries = 3;
    let base_results = 20.min(total_files);
    let collision_multiplier = if filename_collisions > 10 {
        1.5
    } else if filename_collisions > 3 {
        1.25
    } else {
        1.0
    };

    let results_per_query = (base_results as f64 * collision_multiplier).ceil() as usize;
    let tokens_per_result = 15;
    let search_tokens = search_queries * results_per_query * tokens_per_result;

    let noise_tokens = filename_collisions * 25;
    search_tokens + noise_tokens
}

/// Estimates tokens required to read relevant source files.
#[must_use]
pub fn estimate_reading_tokens(total_files: usize, avg_lines: usize) -> usize {
    let files_to_read = 7.min(total_files);
    let avg_chars_per_line = 35.0;
    let avg_file_tokens =
        (avg_lines as f64 * avg_chars_per_line / PYTHON_CHARS_PER_TOKEN).ceil() as usize;
    files_to_read * avg_file_tokens
}

/// Estimates tokens spent following imports and module dependencies.
#[must_use]
pub fn estimate_tracing_tokens(avg_fan_out: f64) -> usize {
    let trace_depth = 2;
    let imports_per_file = (avg_fan_out as usize).clamp(1, 15);
    let tokens_per_trace = 50;
    trace_depth * imports_per_file * tokens_per_trace
}

/// Estimates cognitive comprehension tokens based on complexity and hotspots.
#[must_use]
pub fn estimate_comprehension_tokens(avg_complexity: f64, hotspot_count: usize) -> usize {
    let complexity_overhead = if avg_complexity > 10.0 {
        2000
    } else if avg_complexity > 5.0 {
        1000
    } else {
        500
    };

    let hotspot_overhead = hotspot_count * 150;
    complexity_overhead + hotspot_overhead
}

/// Computes the complete token budget and context status.
#[must_use]
pub fn compute_token_budget(
    total_files: usize,
    total_lines: usize,
    avg_complexity: f64,
    avg_fan_out: f64,
    hotspot_count: usize,
    usable_context: usize,
) -> TokenBudget {
    let avg_lines = total_lines.checked_div(total_files).unwrap_or(0);

    let file_discovery_tokens = estimate_discovery_tokens(total_files, 0);
    let code_reading_tokens = estimate_reading_tokens(total_files, avg_lines);
    let dependency_tracing_tokens = estimate_tracing_tokens(avg_fan_out);
    let comprehension_tokens = estimate_comprehension_tokens(avg_complexity, hotspot_count);

    let total_navigation_tokens = file_discovery_tokens
        + code_reading_tokens
        + dependency_tracing_tokens
        + comprehension_tokens;

    let navigation_pct = if usable_context > 0 {
        (total_navigation_tokens as f64 / usable_context as f64) * 100.0
    } else {
        0.0
    };

    let remaining_tokens = usable_context.saturating_sub(total_navigation_tokens);

    let status_verdict = if navigation_pct < 25.0 {
        ContextStatusVerdict::Generous
    } else if navigation_pct < 50.0 {
        ContextStatusVerdict::Healthy
    } else if navigation_pct < 75.0 {
        ContextStatusVerdict::Moderate
    } else if navigation_pct < 90.0 {
        ContextStatusVerdict::Constrained
    } else {
        ContextStatusVerdict::Exhausted
    };

    TokenBudget {
        file_discovery_tokens,
        code_reading_tokens,
        dependency_tracing_tokens,
        comprehension_tokens,
        total_navigation_tokens,
        usable_context,
        navigation_pct,
        remaining_tokens,
        status_verdict,
    }
}
