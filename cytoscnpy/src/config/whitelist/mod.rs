mod builtin;

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
        let pattern_lower = pattern.to_lowercase();
        let path_lower = path.to_lowercase();

        if pattern_lower.contains("**") {
            let parts: Vec<&str> = pattern_lower.split("**").collect();
            if parts.len() == 2 {
                let prefix = parts[0].trim_end_matches('/');
                let suffix = parts[1].trim_start_matches('/');
                return (prefix.is_empty() || path_lower.starts_with(prefix))
                    && (suffix.is_empty() || path_lower.ends_with(suffix));
            }
        }

        if pattern_lower.contains('*') {
            let mut regex_pattern = String::from("^");
            for ch in pattern_lower.chars() {
                match ch {
                    '*' => regex_pattern.push_str(".*"),
                    '.' | '^' | '$' | '+' | '[' | ']' | '(' | ')' | '{' | '}' | '\\' | '|' => {
                        regex_pattern.push('\\');
                        regex_pattern.push(ch);
                    }
                    _ => regex_pattern.push(ch),
                }
            }
            regex_pattern.push('$');
            return regex::Regex::new(&regex_pattern).is_ok_and(|re| re.is_match(&path_lower));
        }

        path_lower == pattern_lower || path_lower.starts_with(&format!("{pattern_lower}/"))
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
