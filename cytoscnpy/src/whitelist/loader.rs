//! Whitelist loading from Python and TOML files.
//!
//! Supports loading whitelists from:
//! - Python files (Vulture-compatible format)
//! - TOML configuration files
//!
//! # Python Format
//!
//! ```python
//! # Comments are ignored
//! my_function  # Trailing comments are also ignored
//! MyUnusedClass
//! _private_var
//! ```

use std::fs;
use std::io;
use std::path::Path;

use crate::config::{WhitelistEntry, WhitelistPattern};
use crate::whitelist::Whitelist;

/// Load a whitelist from a file.
///
/// Supports:
/// - `.py` files: Python format (Vulture-compatible)
/// - `.toml` files: TOML format
pub fn load_whitelist_file(path: &Path) -> io::Result<Whitelist> {
    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_lowercase);

    match extension.as_deref() {
        Some("py") => load_python_whitelist(path),
        Some("toml") => load_toml_whitelist(path),
        _ => {
            // Try to detect format from content
            let content = fs::read_to_string(path)?;
            if content.trim_start().starts_with('[') {
                load_toml_whitelist_from_str(&content, path)
            } else {
                Ok(load_python_whitelist_from_str(&content))
            }
        }
    }
}

/// Load a whitelist from a Python file.
///
/// The format is simple: each non-comment line is treated as a symbol name.
/// Lines starting with `#` are comments. Trailing `#` comments are stripped.
pub fn load_python_whitelist(path: &Path) -> io::Result<Whitelist> {
    let content = fs::read_to_string(path)?;
    Ok(load_python_whitelist_from_str(&content))
}

/// Load a Python whitelist from a string.
fn load_python_whitelist_from_str(content: &str) -> Whitelist {
    let mut whitelist = Whitelist::new();

    for line in content.lines() {
        // Strip trailing comments
        let line = strip_trailing_comment(line);
        let line = line.trim();

        // Skip empty lines and full-line comments
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Extract the symbol name
        // In Python, valid identifiers are: [a-zA-Z_][a-zA-Z0-9_]*
        // But we also want to support attribute access like "Class.method"
        let name = extract_symbol_name(line);

        if !name.is_empty() {
            whitelist.add_entry(WhitelistEntry {
                name: name.to_owned(),
                pattern: Some(WhitelistPattern::Exact),
                file: None,
                category: None,
            });
        }
    }

    whitelist
}

/// Load a whitelist from a TOML file.
pub fn load_toml_whitelist(path: &Path) -> io::Result<Whitelist> {
    let content = fs::read_to_string(path)?;
    load_toml_whitelist_from_str(&content, path)
}

/// Load a TOML whitelist from a string.
fn load_toml_whitelist_from_str(content: &str, _path: &Path) -> io::Result<Whitelist> {
    let mut whitelist = Whitelist::new();

    // Parse as TOML
    let value: toml::Value =
        toml::from_str(content).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    // Look for whitelist entries in various locations
    // 1. [cytoscnpy.whitelist] array
    // 2. [[whitelist]] array
    // 3. Top-level whitelist array
    let entries = extract_toml_whitelist_entries(&value);

    for entry in entries {
        whitelist.add_entry(entry);
    }

    Ok(whitelist)
}

/// Extract whitelist entries from a TOML value.
fn extract_toml_whitelist_entries(value: &toml::Value) -> Vec<WhitelistEntry> {
    let mut entries = Vec::new();

    // Try [cytoscnpy.whitelist]
    if let Some(cytoscnpy) = value.get("cytoscnpy") {
        if let Some(whitelist) = cytoscnpy.get("whitelist") {
            if let Some(arr) = whitelist.as_array() {
                for item in arr {
                    if let Some(entry) = toml_to_entry(item) {
                        entries.push(entry);
                    }
                }
            }
        }
    }

    // Try [[whitelist]]
    if let Some(whitelist) = value.get("whitelist") {
        if let Some(arr) = whitelist.as_array() {
            for item in arr {
                if let Some(entry) = toml_to_entry(item) {
                    entries.push(entry);
                }
            }
        }
    }

    entries
}

/// Convert a TOML value to a `WhitelistEntry`.
fn toml_to_entry(value: &toml::Value) -> Option<WhitelistEntry> {
    let table = value.as_table()?;

    let name = table.get("name")?.as_str()?.to_owned();
    let pattern = table.get("pattern").and_then(|v| v.as_str()).and_then(|s| {
        match s.to_lowercase().as_str() {
            "exact" => Some(WhitelistPattern::Exact),
            "wildcard" => Some(WhitelistPattern::Wildcard),
            "regex" => Some(WhitelistPattern::Regex),
            _ => None,
        }
    });
    let file = table
        .get("file")
        .and_then(|v| v.as_str())
        .map(str::to_owned);
    let category = table
        .get("category")
        .and_then(|v| v.as_str())
        .map(str::to_owned);

    Some(WhitelistEntry {
        name,
        pattern,
        file,
        category,
    })
}

/// Strip a trailing comment from a line.
fn strip_trailing_comment(line: &str) -> &str {
    // Find the first '#' that's not inside a string
    let mut in_string = false;
    let mut string_char = ' ';
    let mut escape_next = false;

    for (i, ch) in line.char_indices() {
        if escape_next {
            escape_next = false;
            continue;
        }

        match ch {
            '\\' if in_string => escape_next = true,
            '"' | '\'' if !in_string => {
                in_string = true;
                string_char = ch;
            }
            c if in_string && c == string_char => in_string = false,
            '#' if !in_string => return &line[..i],
            _ => {}
        }
    }

    line
}

/// Extract a symbol name from a line.
///
/// Handles:
/// - Simple identifiers: `my_function`
/// - Attribute access: `Class.method`
/// - Dunder methods: `__init__`
fn extract_symbol_name(line: &str) -> &str {
    // Take the first token (up to whitespace or special chars)
    let end = line
        .char_indices()
        .find(|(_, ch)| !is_valid_name_char(*ch))
        .map(|(i, _)| i)
        .unwrap_or(line.len());

    &line[..end]
}

/// Check if a character is valid in a Python identifier.
fn is_valid_name_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_' || ch == '.'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_python_whitelist() {
        let content = r#"
# This is a comment
my_function  # Trailing comment
MyUnusedClass
_private_var

# Another comment
another_function
"#;
        let whitelist = load_python_whitelist_from_str(content);

        assert!(whitelist.is_whitelisted("my_function", None));
        assert!(whitelist.is_whitelisted("MyUnusedClass", None));
        assert!(whitelist.is_whitelisted("_private_var", None));
        assert!(whitelist.is_whitelisted("another_function", None));
        assert!(!whitelist.is_whitelisted("not_in_whitelist", None));
    }

    #[test]
    fn test_strip_trailing_comment() {
        assert_eq!(
            strip_trailing_comment("my_function  # comment"),
            "my_function  "
        );
        assert_eq!(strip_trailing_comment("my_function"), "my_function");
        assert_eq!(strip_trailing_comment("# comment"), "");
        assert_eq!(
            strip_trailing_comment("value = \"#not a comment\""),
            "value = \"#not a comment\""
        );
    }

    #[test]
    fn test_extract_symbol_name() {
        assert_eq!(extract_symbol_name("my_function"), "my_function");
        assert_eq!(extract_symbol_name("my_function  "), "my_function");
        assert_eq!(extract_symbol_name("Class.method"), "Class.method");
        assert_eq!(extract_symbol_name("__init__"), "__init__");
        assert_eq!(extract_symbol_name("my_function#comment"), "my_function");
    }

    #[test]
    fn test_load_toml_whitelist() {
        let content = r#"
[cytoscnpy]
whitelist = [
    { name = "my_function" },
    { name = "test_*", pattern = "wildcard" },
    { name = "api_.*", pattern = "regex" },
]
"#;
        let whitelist = load_toml_whitelist_from_str(content, Path::new("test.toml")).unwrap();

        assert!(whitelist.is_whitelisted("my_function", None));
        assert!(whitelist.is_whitelisted("test_something", None));
        assert!(whitelist.is_whitelisted("api_handler", None));
        assert!(!whitelist.is_whitelisted("other_function", None));
    }

    #[test]
    fn test_empty_whitelist() {
        let content = "";
        let whitelist = load_python_whitelist_from_str(content);
        assert!(whitelist.is_empty());
    }
}
