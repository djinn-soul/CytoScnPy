//! Whitelist matcher for efficient symbol matching.
//!
//! Provides optimized matching against compiled whitelists.

use crate::config::{get_builtin_whitelists, WhitelistEntry};
use crate::whitelist::Whitelist;

/// A matcher that combines built-in and user whitelists.
#[derive(Debug)]
pub struct WhitelistMatcher {
    /// Built-in whitelists for common Python modules.
    builtin: Whitelist,
    /// User-defined whitelists from config files.
    user: Whitelist,
    /// Whitelists loaded from external files.
    external: Whitelist,
}

impl WhitelistMatcher {
    /// Create a new matcher with built-in whitelists.
    pub fn new() -> Self {
        let builtin = Whitelist::from_entries(get_builtin_whitelists());
        Self {
            builtin,
            user: Whitelist::new(),
            external: Whitelist::new(),
        }
    }

    /// Create a matcher with user-defined whitelist entries.
    pub fn with_user_entries(entries: Vec<WhitelistEntry>) -> Self {
        let builtin = Whitelist::from_entries(get_builtin_whitelists());
        let user = Whitelist::from_entries(entries);
        Self {
            builtin,
            user,
            external: Whitelist::new(),
        }
    }

    /// Add user-defined whitelist entries.
    pub fn add_user_entries(&mut self, entries: Vec<WhitelistEntry>) {
        for entry in entries {
            self.user.add_entry(entry);
        }
    }

    /// Add an external whitelist (loaded from a file).
    pub fn add_external(&mut self, whitelist: Whitelist) {
        self.external.merge(whitelist);
    }

    /// Check if a symbol is whitelisted.
    ///
    /// Checks in order:
    /// 1. User-defined whitelists (highest priority)
    /// 2. External whitelists
    /// 3. Built-in whitelists (lowest priority)
    pub fn is_whitelisted(&self, name: &str, file_path: Option<&str>) -> bool {
        // Check user-defined first (highest priority)
        if self.user.is_whitelisted(name, file_path) {
            return true;
        }

        // Check external whitelists
        if self.external.is_whitelisted(name, file_path) {
            return true;
        }

        // Check built-in whitelists
        self.builtin.is_whitelisted(name, file_path)
    }

    /// Get the total number of whitelist entries.
    pub fn total_entries(&self) -> usize {
        self.builtin.len() + self.user.len() + self.external.len()
    }

    /// Check if there are any user-defined entries.
    pub fn has_user_entries(&self) -> bool {
        !self.user.is_empty()
    }

    /// Check if there are any external entries.
    pub fn has_external_entries(&self) -> bool {
        !self.external.is_empty()
    }
}

impl Default for WhitelistMatcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::WhitelistPattern;

    #[test]
    fn test_builtin_whitelist() {
        let matcher = WhitelistMatcher::new();

        // Built-in entries should be matched
        assert!(matcher.is_whitelisted("add_argument", None));
        assert!(matcher.is_whitelisted("getLogger", None));
        assert!(matcher.is_whitelisted("setUp", None));
    }

    #[test]
    fn test_user_whitelist() {
        let matcher = WhitelistMatcher::with_user_entries(vec![WhitelistEntry {
            name: "my_function".into(),
            pattern: Some(WhitelistPattern::Exact),
            file: None,
            category: None,
        }]);

        assert!(matcher.is_whitelisted("my_function", None));
        assert!(matcher.is_whitelisted("add_argument", None)); // Built-in still works
    }

    #[test]
    fn test_external_whitelist() {
        let mut matcher = WhitelistMatcher::new();
        let mut external = Whitelist::new();
        external.add_entry(WhitelistEntry {
            name: "external_func".into(),
            pattern: Some(WhitelistPattern::Exact),
            file: None,
            category: None,
        });
        matcher.add_external(external);

        assert!(matcher.is_whitelisted("external_func", None));
        assert!(matcher.is_whitelisted("add_argument", None)); // Built-in still works
    }

    #[test]
    fn test_priority() {
        // User entries should take priority
        let matcher = WhitelistMatcher::with_user_entries(vec![WhitelistEntry {
            name: "test_*".into(),
            pattern: Some(WhitelistPattern::Wildcard),
            file: None,
            category: None,
        }]);

        assert!(matcher.is_whitelisted("test_function", None));
    }
}
