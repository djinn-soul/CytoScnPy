//! Filtering heuristics and word boundary matching for unreferenced function analysis.

/// Common framework hook prefixes and lifecycle methods that are called via
/// `IoC`, dynamic dispatch, or convention rather than explicit user calls.
pub const FRAMEWORK_PREFIXES: &[&str] = &[
    "get",
    "set",
    "is",
    "has",
    "on",
    "handle",
    "configure",
    "build",
    "create",
    "update",
    "delete",
    "remove",
    "find",
    "to",
    "from",
    "supports",
    "execute",
    "invoke",
    "process",
    "render",
    "validate",
    "resolve",
    "load",
    "register",
    "subscribe",
    "provide",
    "apply",
    "normalize",
    "transform",
    "before",
    "after",
    "can",
    "should",
    "parse",
    "format",
    "serialize",
    "deserialize",
    "map",
    "filter",
    "run",
    "start",
    "stop",
    "boot",
    "init",
    "setup",
    "teardown",
    "main",
];

/// Options controlling unreferenced function heuristics.
#[derive(Debug, Clone)]
pub struct UnreferencedOptions {
    /// Minimum physical lines for a function to be considered large (default: 15).
    pub min_lines: usize,
    /// Minimum function name length to avoid trivial names (default: 8).
    pub min_name_len: usize,
    /// Whether to include test files in scanning.
    pub include_tests: bool,
}

impl Default for UnreferencedOptions {
    fn default() -> Self {
        Self {
            min_lines: 15,
            min_name_len: 8,
            include_tests: false,
        }
    }
}

/// Evaluates whether a function candidate should be skipped according to conservative heuristics.
#[must_use]
pub fn should_skip_function(name: &str, line_count: usize, options: &UnreferencedOptions) -> bool {
    if name == "<anonymous>" || name == "main" || name.starts_with("__") {
        return true;
    }

    // Skip test functions
    if name.starts_with("test_")
        || name.starts_with("Test")
        || name.starts_with("test")
        || name.ends_with("_test")
    {
        return true;
    }

    // Skip small functions (below line threshold)
    if line_count < options.min_lines {
        return true;
    }

    // Skip short names (too likely to be general helpers or common idioms)
    if name.len() < options.min_name_len {
        return true;
    }

    // Skip names starting with framework hook prefixes
    let lower = name.to_lowercase();
    for &prefix in FRAMEWORK_PREFIXES {
        if lower.starts_with(prefix) {
            // Check boundary: either exact match, followed by '_', or followed by an uppercase letter in original
            let prefix_len = prefix.len();
            if lower.len() == prefix_len
                || name.as_bytes().get(prefix_len) == Some(&b'_')
                || (name
                    .as_bytes()
                    .get(prefix_len)
                    .is_some_and(u8::is_ascii_uppercase))
            {
                return true;
            }
        }
    }

    false
}

/// Returns true if `b` is an ASCII identifier character (`[a-zA-Z0-9_]`).
#[must_use]
pub const fn is_ident_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// Checks if `needle` appears in `haystack` as a distinct identifier token.
#[must_use]
pub fn contains_as_word(haystack: &str, needle: &str) -> bool {
    let needle_bytes = needle.as_bytes();
    let hay_bytes = haystack.as_bytes();
    if needle_bytes.len() > hay_bytes.len() || needle_bytes.is_empty() {
        return false;
    }

    let mut start = 0;
    while let Some(pos) = haystack[start..].find(needle) {
        let abs = start + pos;
        let before_ok = abs == 0 || !is_ident_char(hay_bytes[abs - 1]);
        let after_pos = abs + needle_bytes.len();
        let after_ok = after_pos >= hay_bytes.len() || !is_ident_char(hay_bytes[after_pos]);

        if before_ok && after_ok {
            return true;
        }
        start = abs + 1;
    }
    false
}
