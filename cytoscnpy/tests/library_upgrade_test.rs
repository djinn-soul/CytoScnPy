//! Integration tests for verifying library upgrades.
//!
//! These tests ensure that upgraded dependencies work correctly with the codebase.
//! Run after updating dependencies in Cargo.toml to validate compatibility.

use cytoscnpy::config::Config;
use std::fs;
use std::path::Path;

// ============================================================================
// TOML Library Tests (toml 0.9 upgrade)
// ============================================================================

/// Test basic TOML parsing with the upgraded toml crate.
#[test]
fn test_toml_upgrade_basic_parsing() {
    let test_dir = Path::new("test_toml_upgrade_basic");
    setup_test_dir(test_dir);

    let content = r#"
[cytoscnpy]
confidence = 85
exclude_folders = ["node_modules", ".venv"]
include_tests = false
"#;
    fs::write(test_dir.join(".cytoscnpy.toml"), content).unwrap();

    let config = Config::load_from_path(test_dir);

    assert_eq!(config.cytoscnpy.confidence, Some(85));
    assert_eq!(
        config.cytoscnpy.exclude_folders,
        Some(vec!["node_modules".to_owned(), ".venv".to_owned()])
    );
    assert_eq!(config.cytoscnpy.include_tests, Some(false));

    cleanup_test_dir(test_dir);
}

/// Test TOML parsing with all optional fields.
#[test]
fn test_toml_upgrade_all_fields() {
    let test_dir = Path::new("test_toml_upgrade_all_fields");
    setup_test_dir(test_dir);

    let content = r#"
[cytoscnpy]
confidence = 100
exclude_folders = ["a", "b", "c"]
include_folders = ["src", "lib"]
include_tests = true
secrets = true
danger = true
quality = true
max_lines = 500
max_args = 10
complexity = 15
nesting = 5
ignore = ["CSP-D001", "CSP-Q002"]
"#;
    fs::write(test_dir.join(".cytoscnpy.toml"), content).unwrap();

    let config = Config::load_from_path(test_dir);

    assert_eq!(config.cytoscnpy.confidence, Some(100));
    assert_eq!(config.cytoscnpy.include_tests, Some(true));
    assert_eq!(config.cytoscnpy.secrets, Some(true));
    assert_eq!(config.cytoscnpy.danger, Some(true));
    assert_eq!(config.cytoscnpy.quality, Some(true));
    assert_eq!(config.cytoscnpy.max_lines, Some(500));
    assert_eq!(config.cytoscnpy.max_args, Some(10));
    assert_eq!(config.cytoscnpy.complexity, Some(15));
    assert_eq!(config.cytoscnpy.nesting, Some(5));
    assert_eq!(
        config.cytoscnpy.ignore,
        Some(vec!["CSP-D001".to_owned(), "CSP-Q002".to_owned()])
    );

    cleanup_test_dir(test_dir);
}

/// Test TOML parsing with empty arrays.
#[test]
fn test_toml_upgrade_empty_arrays() {
    let test_dir = Path::new("test_toml_upgrade_empty_arrays");
    setup_test_dir(test_dir);

    let content = r"
[cytoscnpy]
exclude_folders = []
ignore = []
";
    fs::write(test_dir.join(".cytoscnpy.toml"), content).unwrap();

    let config = Config::load_from_path(test_dir);

    assert_eq!(config.cytoscnpy.exclude_folders, Some(vec![]));
    assert_eq!(config.cytoscnpy.ignore, Some(vec![]));

    cleanup_test_dir(test_dir);
}

/// Test pyproject.toml parsing (nested tool section).
#[test]
fn test_toml_upgrade_pyproject_nested() {
    let test_dir = Path::new("test_toml_upgrade_pyproject");
    setup_test_dir(test_dir);

    let content = r#"
[project]
name = "myproject"
version = "1.0.0"

[tool.black]
line-length = 88

[tool.cytoscnpy]
confidence = 70
secrets = false
max_lines = 300
"#;
    fs::write(test_dir.join("pyproject.toml"), content).unwrap();

    let config = Config::load_from_path(test_dir);

    assert_eq!(config.cytoscnpy.confidence, Some(70));
    assert_eq!(config.cytoscnpy.secrets, Some(false));
    assert_eq!(config.cytoscnpy.max_lines, Some(300));

    cleanup_test_dir(test_dir);
}

/// Test TOML parsing with unicode strings.
#[test]
fn test_toml_upgrade_unicode() {
    let test_dir = Path::new("test_toml_upgrade_unicode");
    setup_test_dir(test_dir);

    let content = r#"
[cytoscnpy]
exclude_folders = ["日本語フォルダ", "目录", "Ordner"]
"#;
    fs::write(test_dir.join(".cytoscnpy.toml"), content).unwrap();

    let config = Config::load_from_path(test_dir);

    assert_eq!(
        config.cytoscnpy.exclude_folders,
        Some(vec![
            "日本語フォルダ".to_owned(),
            "目录".to_owned(),
            "Ordner".to_owned()
        ])
    );

    cleanup_test_dir(test_dir);
}

/// Test TOML parsing with special characters in strings.
#[test]
fn test_toml_upgrade_special_chars() {
    let test_dir = Path::new("test_toml_upgrade_special");
    setup_test_dir(test_dir);

    let content = r#"
[cytoscnpy]
exclude_folders = ["path/with/slashes", "path\\with\\backslashes", "has spaces"]
"#;
    fs::write(test_dir.join(".cytoscnpy.toml"), content).unwrap();

    let config = Config::load_from_path(test_dir);

    assert_eq!(
        config.cytoscnpy.exclude_folders,
        Some(vec![
            "path/with/slashes".to_owned(),
            "path\\with\\backslashes".to_owned(),
            "has spaces".to_owned()
        ])
    );

    cleanup_test_dir(test_dir);
}

// ============================================================================
// Colored Library Tests (colored 3.0 upgrade)
// ============================================================================

/// Test that colored output can be disabled (important for testing).
#[test]
fn test_colored_upgrade_overrides() {
    // 1. Test disabling colors
    colored::control::set_override(false);

    use colored::Colorize;
    let text_no_color = "test".red().to_string();

    // When colors are disabled, no ANSI codes should be present
    assert!(!text_no_color.contains("\x1b["));
    assert_eq!(text_no_color, "test");

    // 2. Test enabling colors
    colored::control::set_override(true);

    let text_with_color = "test".red().to_string();

    // Text should contain the word "test"
    assert!(text_with_color.contains("test"));

    // Cleanup
    colored::control::unset_override();
}

/// Test that colored strings can be created (API compatibility check).
#[test]
fn test_colored_upgrade_api_compatibility() {
    use colored::Colorize;

    // These should compile and not panic - API compatibility check
    let _red = "test".red();
    let _green = "test".green();
    let _blue = "test".blue();
    let _bold = "test".bold();
    let _underline = "test".underline();
    let _chained = "test".red().bold().underline();
    let _bg = "test".on_red();

    // All should be convertible to String
    assert!(!_red.to_string().is_empty());
    assert!(!_green.to_string().is_empty());
    assert!(!_chained.to_string().is_empty());
}

/// Test various color methods produce output (regardless of terminal support).
#[test]
fn test_colored_upgrade_various_colors() {
    use colored::Colorize;

    // Test that colored methods produce non-empty strings
    let red = "red".red().to_string();
    let green = "green".green().to_string();
    let blue = "blue".blue().to_string();
    let yellow = "yellow".yellow().to_string();
    let cyan = "cyan".cyan().to_string();
    let magenta = "magenta".magenta().to_string();

    // All should contain at least the original text
    assert!(red.contains("red"));
    assert!(green.contains("green"));
    assert!(blue.contains("blue"));
    assert!(yellow.contains("yellow"));
    assert!(cyan.contains("cyan"));
    assert!(magenta.contains("magenta"));
}

/// Test style methods (bold, italic, underline) produce output.
#[test]
fn test_colored_upgrade_styles() {
    use colored::Colorize;

    let bold = "bold".bold().to_string();
    let underline = "underline".underline().to_string();
    let italic = "italic".italic().to_string();
    let dimmed = "dimmed".dimmed().to_string();

    // All should contain the original text
    assert!(bold.contains("bold"));
    assert!(underline.contains("underline"));
    assert!(italic.contains("italic"));
    assert!(dimmed.contains("dimmed"));
}

/// Test chained color and style produces output.
#[test]
fn test_colored_upgrade_chained_styles() {
    use colored::Colorize;

    let styled = "styled".red().bold().underline().to_string();

    // Should contain the original text
    assert!(styled.contains("styled"));
}

/// Test background colors produce output.
#[test]
fn test_colored_upgrade_background() {
    use colored::Colorize;

    let bg_red = "bg".on_red().to_string();
    let bg_green = "bg".on_green().to_string();
    let bg_blue = "bg".on_blue().to_string();

    // All should contain the original text
    assert!(bg_red.contains("bg"));
    assert!(bg_green.contains("bg"));
    assert!(bg_blue.contains("bg"));
}

// ============================================================================
// Helper Functions
// ============================================================================

fn setup_test_dir(path: &Path) {
    if path.exists() {
        fs::remove_dir_all(path).unwrap();
    }
    fs::create_dir(path).unwrap();
}

fn cleanup_test_dir(path: &Path) {
    if path.exists() {
        fs::remove_dir_all(path).unwrap();
    }
}
