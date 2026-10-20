//! File-restricted whitelist entries with their patterns compiled once.

use crate::config::{WhitelistEntry, WhitelistPattern};

/// A file-restricted entry with its patterns compiled once. Matches exactly
/// like [`WhitelistEntry::matches`].
#[derive(Debug, Clone)]
pub(super) struct FileSpecificEntry {
    /// `None` when the file pattern is invalid, which never matches.
    file: Option<globset::GlobMatcher>,
    name: String,
    pattern: WhitelistPattern,
    /// Compiled wildcard or regex pattern; `None` for exact entries.
    name_regex: Option<regex::Regex>,
}

impl FileSpecificEntry {
    pub(super) fn new(entry: WhitelistEntry) -> Self {
        Self {
            file: entry.file.as_deref().and_then(WhitelistEntry::file_matcher),
            name_regex: entry.name_regex(),
            pattern: entry.pattern.unwrap_or_default(),
            name: entry.name,
        }
    }

    pub(super) fn matches(&self, symbol_name: &str, file_path: Option<&str>) -> bool {
        let (Some(file), Some(path)) = (&self.file, file_path) else {
            return false;
        };
        if !WhitelistEntry::file_matches(file, path) {
            return false;
        }
        match self.pattern {
            WhitelistPattern::Exact => self.name == symbol_name,
            WhitelistPattern::Wildcard | WhitelistPattern::Regex => self
                .name_regex
                .as_ref()
                .is_some_and(|re| re.is_match(symbol_name)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::FileSpecificEntry;
    use crate::config::{WhitelistEntry, WhitelistPattern};

    #[test]
    fn compiled_entries_match_like_whitelist_entries() {
        let entries = [
            ("process_*", WhitelistPattern::Wildcard, "**/api/*.py"),
            (
                "^validate_.*$",
                WhitelistPattern::Regex,
                "src\\workers\\*.py",
            ),
            ("helper", WhitelistPattern::Exact, "**/UTILS/*.py"),
            ("x[", WhitelistPattern::Regex, "**/*.py"),
            ("*", WhitelistPattern::Wildcard, "[bad"),
        ];
        let names = ["process_data", "validate_input", "helper", "x[", "other"];
        let paths = [
            Some("src/api/users.py"),
            Some("src\\workers\\jobs.py"),
            Some("pkg/utils/helpers.py"),
            Some("pkg/other.py"),
            None,
        ];
        for (name, pattern, file) in entries {
            let entry = WhitelistEntry {
                name: name.to_owned(),
                pattern: Some(pattern),
                file: Some(file.to_owned()),
                category: None,
            };
            let compiled = FileSpecificEntry::new(entry.clone());
            for symbol in names {
                for path in paths {
                    assert_eq!(
                        compiled.matches(symbol, path),
                        entry.matches(symbol, path),
                        "{name:?} in {file:?} vs {symbol:?} at {path:?}"
                    );
                }
            }
        }
    }
}
