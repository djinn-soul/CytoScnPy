mod builtin;

use globset::GlobBuilder;
use serde::Deserialize;

/// A whitelist entry for ignoring false positives in dead code detection.
#[derive(Debug, Deserialize, Clone)]
pub struct WhitelistEntry {
    /// The symbol name or pattern to whitelist.
    pub name: String,

    /// The type of pattern matching to use.
    /// - `exact` (default): Match the name exactly
    /// - `wildcard`: Use glob-style wildcards (e.g., `test_*`)
    /// - `regex`: Use regular expressions
    #[serde(default)]
    pub pattern: Option<WhitelistPattern>,

    /// Optional file path to restrict the whitelist to a specific file.
    /// Supports glob patterns (e.g., `src/api/*.py`).
    #[serde(default)]
    pub file: Option<String>,

    /// Optional category for documentation/organization purposes.
    #[serde(default)]
    pub category: Option<String>,
}

/// Pattern matching type for whitelist entries.
#[derive(Debug, Deserialize, Clone, Copy, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum WhitelistPattern {
    /// Exact string match (default).
    #[default]
    Exact,
    /// Glob-style wildcard matching (e.g., `test_*`, `*_handler`).
    Wildcard,
    /// Regular expression matching.
    Regex,
}

impl WhitelistEntry {
    /// Check if a symbol name matches this whitelist entry.
    #[must_use]
    pub fn matches(&self, symbol_name: &str, file_path: Option<&str>) -> bool {
        if let Some(ref file_pattern) = self.file {
            if let Some(path) = file_path {
                if !Self::matches_file_pattern(file_pattern, path) {
                    return false;
                }
            } else {
                return false;
            }
        }

        match self.pattern.unwrap_or_default() {
            WhitelistPattern::Exact => self.name == symbol_name,
            WhitelistPattern::Wildcard => self.matches_wildcard(symbol_name),
            WhitelistPattern::Regex => self.matches_regex(symbol_name),
        }
    }

    fn matches_wildcard(&self, symbol_name: &str) -> bool {
        let mut regex_pattern = String::from("^");
        for ch in self.name.chars() {
            match ch {
                '*' => regex_pattern.push_str(".*"),
                '?' => regex_pattern.push('.'),
                '.' | '^' | '$' | '+' | '[' | ']' | '(' | ')' | '{' | '}' | '\\' | '|' => {
                    regex_pattern.push('\\');
                    regex_pattern.push(ch);
                }
                _ => regex_pattern.push(ch),
            }
        }
        regex_pattern.push('$');

        match regex::Regex::new(&regex_pattern) {
            Ok(re) => re.is_match(symbol_name),
            Err(_) => false,
        }
    }

    fn matches_regex(&self, symbol_name: &str) -> bool {
        match regex::Regex::new(&self.name) {
            Ok(re) => re.is_match(symbol_name),
            Err(_) => false,
        }
    }

    fn matches_file_pattern(pattern: &str, path: &str) -> bool {
        let normalized_pattern = pattern.replace('\\', "/");
        let normalized_path = path.replace('\\', "/");

        let Ok(glob) = GlobBuilder::new(&normalized_pattern)
            .case_insensitive(true)
            .build()
        else {
            return false;
        };

        glob.compile_matcher().is_match(normalized_path)
    }
}

/// Returns built-in default whitelists for common Python modules.
#[must_use]
pub fn get_builtin_whitelists() -> Vec<WhitelistEntry> {
    builtin::entries()
}

impl Default for WhitelistEntry {
    fn default() -> Self {
        Self {
            name: String::new(),
            pattern: Some(WhitelistPattern::Exact),
            file: None,
            category: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::WhitelistEntry;

    #[test]
    fn file_pattern_supports_double_star_with_suffix_wildcard() {
        assert!(WhitelistEntry::matches_file_pattern(
            "**/api/*.py",
            "src/app/api/users.py"
        ));
        assert!(!WhitelistEntry::matches_file_pattern(
            "**/api/*.py",
            "src/app/apis/users.py"
        ));
    }

    #[test]
    fn file_pattern_is_case_insensitive_and_separator_agnostic() {
        assert!(WhitelistEntry::matches_file_pattern(
            "SRC/**/API/*.PY",
            r"src\core\api\handler.py"
        ));
    }
}
