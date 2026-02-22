//! Whitelist management for false positive suppression.
//!
//! This module provides functionality for:
//! - Generating whitelists from detected unused code (similar to Vulture's `--make-whitelist`)
//! - Loading whitelists from Python files or TOML configuration
//! - Matching symbols against whitelist entries
//!
//! # Example: Generate a whitelist
//!
//! ```bash
//! cytoscnpy src/ --make-whitelist > whitelist.py
//! cytoscnpy src/ --whitelist whitelist.py
//! ```
//!
//! # Whitelist File Formats
//!
//! ## Python Format (Vulture-compatible)
//!
//! ```python
//! # Whitelist for my_project
//! my_unused_function  # function
//! MyUnusedClass  # class
//! unused_variable  # variable
//! ```
//!
//! ## TOML Format (in .cytoscnpy.toml)
//!
//! ```toml
//! [cytoscnpy]
//! whitelist = [
//!     { name = "my_plugin_hook" },
//!     { name = "test_*", pattern = "wildcard" },
//! ]
//! ```

mod generator;
mod loader;
mod matcher;

pub use generator::generate_whitelist;
pub use loader::load_whitelist_file;
pub use matcher::WhitelistMatcher;

use crate::config::{WhitelistEntry, WhitelistPattern};
use rustc_hash::FxHashSet;

/// A compiled whitelist for efficient matching.
#[derive(Debug, Clone, Default)]
pub struct Whitelist {
    /// Exact name matches.
    exact_names: FxHashSet<String>,
    /// Wildcard patterns (converted to regex).
    wildcard_patterns: Vec<(String, regex::Regex)>,
    /// Regex patterns.
    regex_patterns: Vec<(String, regex::Regex)>,
    /// File-specific entries.
    file_specific: Vec<WhitelistEntry>,
}

impl Whitelist {
    /// Create a new empty whitelist.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a whitelist from a list of entries.
    pub fn from_entries(entries: Vec<WhitelistEntry>) -> Self {
        let mut whitelist = Self::new();
        for entry in entries {
            whitelist.add_entry(entry);
        }
        whitelist
    }

    /// Add an entry to the whitelist.
    pub fn add_entry(&mut self, entry: WhitelistEntry) {
        // If there's a file restriction, store the full entry
        if entry.file.is_some() {
            self.file_specific.push(entry);
            return;
        }

        match entry.pattern.unwrap_or_default() {
            WhitelistPattern::Exact => {
                self.exact_names.insert(entry.name);
            }
            WhitelistPattern::Wildcard => {
                if let Ok(re) = wildcard_to_regex(&entry.name) {
                    self.wildcard_patterns.push((entry.name.clone(), re));
                }
            }
            WhitelistPattern::Regex => {
                if let Ok(re) = regex::Regex::new(&entry.name) {
                    self.regex_patterns.push((entry.name.clone(), re));
                }
            }
        }
    }

    /// Check if a symbol name is whitelisted.
    ///
    /// # Arguments
    /// * `name` - The symbol name to check.
    /// * `file_path` - Optional file path for file-specific whitelisting.
    ///
    /// # Returns
    /// `true` if the symbol matches any whitelist entry.
    pub fn is_whitelisted(&self, name: &str, file_path: Option<&str>) -> bool {
        // Check exact matches first (fastest)
        if self.exact_names.contains(name) {
            return true;
        }

        // Check wildcard patterns
        for (_, pattern) in &self.wildcard_patterns {
            if pattern.is_match(name) {
                return true;
            }
        }

        // Check regex patterns
        for (_, pattern) in &self.regex_patterns {
            if pattern.is_match(name) {
                return true;
            }
        }

        // Check file-specific entries
        for entry in &self.file_specific {
            if entry.matches(name, file_path) {
                return true;
            }
        }

        false
    }

    /// Merge another whitelist into this one.
    pub fn merge(&mut self, other: Whitelist) {
        self.exact_names.extend(other.exact_names);
        self.wildcard_patterns.extend(other.wildcard_patterns);
        self.regex_patterns.extend(other.regex_patterns);
        self.file_specific.extend(other.file_specific);
    }

    /// Get the total number of whitelist entries.
    pub fn len(&self) -> usize {
        self.exact_names.len()
            + self.wildcard_patterns.len()
            + self.regex_patterns.len()
            + self.file_specific.len()
    }

    /// Check if the whitelist is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Convert a wildcard pattern to a regex.
///
/// Supports:
/// - `*` matches any characters
/// - `?` matches a single character
fn wildcard_to_regex(pattern: &str) -> Result<regex::Regex, regex::Error> {
    let mut regex_pattern = String::with_capacity(pattern.len() * 2);
    regex_pattern.push('^');

    for ch in pattern.chars() {
        match ch {
            '*' => regex_pattern.push_str(".*"),
            '?' => regex_pattern.push('.'),
            // Escape regex special characters
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exact_match() {
        let mut whitelist = Whitelist::new();
        whitelist.add_entry(WhitelistEntry {
            name: "my_function".into(),
            pattern: Some(WhitelistPattern::Exact),
            file: None,
            category: None,
        });

        assert!(whitelist.is_whitelisted("my_function", None));
        assert!(!whitelist.is_whitelisted("other_function", None));
    }

    #[test]
    fn test_wildcard_match() {
        let mut whitelist = Whitelist::new();
        whitelist.add_entry(WhitelistEntry {
            name: "test_*".into(),
            pattern: Some(WhitelistPattern::Wildcard),
            file: None,
            category: None,
        });

        assert!(whitelist.is_whitelisted("test_function", None));
        assert!(whitelist.is_whitelisted("test_", None));
        assert!(!whitelist.is_whitelisted("my_test", None));
    }

    #[test]
    fn test_regex_match() {
        let mut whitelist = Whitelist::new();
        whitelist.add_entry(WhitelistEntry {
            name: r"api_\w+_handler".into(),
            pattern: Some(WhitelistPattern::Regex),
            file: None,
            category: None,
        });

        assert!(whitelist.is_whitelisted("api_get_handler", None));
        assert!(whitelist.is_whitelisted("api_post_handler", None));
        assert!(!whitelist.is_whitelisted("api_handler", None));
    }

    #[test]
    fn test_wildcard_to_regex() {
        let re = wildcard_to_regex("test_*").unwrap();
        assert!(re.is_match("test_function"));
        assert!(re.is_match("test_"));
        assert!(!re.is_match("my_test"));

        let re = wildcard_to_regex("*_handler").unwrap();
        assert!(re.is_match("api_handler"));
        assert!(re.is_match("request_handler"));
        assert!(!re.is_match("handler"));
    }
}
