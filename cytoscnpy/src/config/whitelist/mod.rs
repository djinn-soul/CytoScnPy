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
    ///
    /// Invalid patterns never match; call [`WhitelistEntry::validate`] first
    /// to report them.
    #[must_use]
    pub fn matches(&self, symbol_name: &str, file_path: Option<&str>) -> bool {
        if let Some(ref file_pattern) = self.file {
            let Some(path) = file_path else {
                return false;
            };
            let Some(glob) = Self::file_matcher(file_pattern) else {
                return false;
            };
            if !Self::file_matches(&glob, path) {
                return false;
            }
        }
        let pattern = self.pattern.unwrap_or_default();
        Self::name_matches(pattern, &self.name, self.name_regex().as_ref(), symbol_name)
    }

    /// Checks that the name pattern and the optional `file` glob compile.
    ///
    /// # Errors
    /// Returns an error naming the entry when either pattern is invalid.
    pub fn validate(&self) -> anyhow::Result<()> {
        self.compile_name_regex()
            .map_err(|err| anyhow::anyhow!("invalid whitelist pattern '{}': {err}", self.name))?;
        if let Some(file) = &self.file {
            Self::compile_file_matcher(file).map_err(|err| {
                anyhow::anyhow!(
                    "invalid whitelist file pattern '{file}' for '{}': {err}",
                    self.name
                )
            })?;
        }
        Ok(())
    }

    /// Whether `symbol_name` matches a name pattern, given its compiled
    /// regex (`None` for exact entries or an invalid pattern).
    pub(crate) fn name_matches(
        pattern: WhitelistPattern,
        name: &str,
        regex: Option<&regex::Regex>,
        symbol_name: &str,
    ) -> bool {
        match pattern {
            WhitelistPattern::Exact => name == symbol_name,
            WhitelistPattern::Wildcard | WhitelistPattern::Regex => {
                regex.is_some_and(|re| re.is_match(symbol_name))
            }
        }
    }

    /// The compiled name pattern of a wildcard or regex entry, or `None`
    /// for exact entries and invalid patterns (see [`Self::validate`]).
    pub(crate) fn name_regex(&self) -> Option<regex::Regex> {
        self.compile_name_regex().ok().flatten()
    }

    fn compile_name_regex(&self) -> Result<Option<regex::Regex>, regex::Error> {
        match self.pattern.unwrap_or_default() {
            WhitelistPattern::Exact => Ok(None),
            WhitelistPattern::Wildcard => wildcard_to_regex(&self.name).map(Some),
            WhitelistPattern::Regex => regex::Regex::new(&self.name).map(Some),
        }
    }

    /// The compiled `file` pattern, or `None` if it is invalid
    /// (see [`Self::validate`]).
    pub(crate) fn file_matcher(pattern: &str) -> Option<globset::GlobMatcher> {
        Self::compile_file_matcher(pattern).ok()
    }

    fn compile_file_matcher(pattern: &str) -> Result<globset::GlobMatcher, globset::Error> {
        GlobBuilder::new(&pattern.replace('\\', "/"))
            .case_insensitive(true)
            .build()
            .map(|glob| glob.compile_matcher())
    }

    /// Whether `path` matches a compiled `file` pattern, on either separator.
    pub(crate) fn file_matches(glob: &globset::GlobMatcher, path: &str) -> bool {
        glob.is_match(path.replace('\\', "/"))
    }
}

/// Converts a wildcard pattern (`*` any characters, `?` one character) to an
/// anchored regex.
pub(crate) fn wildcard_to_regex(pattern: &str) -> Result<regex::Regex, regex::Error> {
    let mut regex_pattern = String::with_capacity(pattern.len() * 2 + 2);
    regex_pattern.push('^');
    for ch in pattern.chars() {
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
    regex::Regex::new(&regex_pattern)
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
    use super::{WhitelistEntry, WhitelistPattern};

    fn file_matches(pattern: &str, path: &str) -> bool {
        WhitelistEntry::file_matcher(pattern)
            .is_some_and(|glob| WhitelistEntry::file_matches(&glob, path))
    }

    fn entry(name: &str, pattern: WhitelistPattern, file: Option<&str>) -> WhitelistEntry {
        WhitelistEntry {
            name: name.to_owned(),
            pattern: Some(pattern),
            file: file.map(str::to_owned),
            category: None,
        }
    }

    #[test]
    fn file_pattern_supports_double_star_with_suffix_wildcard() {
        assert!(file_matches("**/api/*.py", "src/app/api/users.py"));
        assert!(!file_matches("**/api/*.py", "src/app/apis/users.py"));
    }

    #[test]
    fn file_pattern_is_case_insensitive_and_separator_agnostic() {
        assert!(file_matches("SRC/**/API/*.PY", r"src\core\api\handler.py"));
    }

    #[test]
    fn validate_accepts_valid_patterns() {
        for e in [
            entry("helper", WhitelistPattern::Exact, None),
            entry("test_*", WhitelistPattern::Wildcard, Some("**/tests/*.py")),
            entry("^on_[a-z]+$", WhitelistPattern::Regex, None),
        ] {
            assert!(e.validate().is_ok(), "{e:?}");
        }
    }

    #[test]
    fn validate_rejects_invalid_regex_with_entry_name() {
        let err = entry("keep_(me", WhitelistPattern::Regex, None)
            .validate()
            .expect_err("unclosed group must be rejected");
        assert!(err.to_string().contains("keep_(me"), "{err}");
    }

    #[test]
    fn validate_rejects_invalid_file_glob() {
        let err = entry("x", WhitelistPattern::Exact, Some("[bad"))
            .validate()
            .expect_err("unclosed class must be rejected");
        assert!(err.to_string().contains("[bad"), "{err}");
    }
}
