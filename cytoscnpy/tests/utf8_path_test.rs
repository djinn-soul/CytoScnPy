//! Tests for handling non-UTF-8 file paths gracefully.
//!
//! This test verifies that the analyzer doesn't panic when given
//! paths that contain invalid UTF-8 sequences.

use cytoscnpy::analyzer::CytoScnPy;
use std::path::PathBuf;

/// Test that analyze_code handles non-UTF-8 paths without panicking.
///
/// This test is Unix-only because Windows paths use UTF-16 internally
/// and the `OsStrExt::from_bytes` trait is not available on Windows.
#[cfg(unix)]
#[test]
fn test_no_crash_on_invalid_utf8_path() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    let analyzer = CytoScnPy::default();

    // Create an invalid UTF-8 path.
    // 0xFF is invalid in UTF-8 encoding.
    let bytes = b"invalid_path_\xff.py";
    let os_str = OsStr::from_bytes(bytes);
    let path = PathBuf::from(os_str);

    // This should NOT panic. If it panics, the test framework will catch it.
    // The analyzer should gracefully handle the path (e.g., using to_string_lossy).
    let _result = analyzer.analyze_code("print('hello')", path);
}

/// Test that analyze_code works with normal UTF-8 paths (cross-platform).
#[test]
fn test_valid_utf8_path() {
    let analyzer = CytoScnPy::default();
    let path = PathBuf::from("valid_path.py");

    // This should work without any issues
    let result = analyzer.analyze_code("print('hello')", path);

    // The result should be valid (no parse errors for valid Python)
    assert!(result.parse_errors.is_empty());
}

/// Test that analyze_code handles paths with unicode characters.
#[test]
fn test_unicode_path() {
    let analyzer = CytoScnPy::default();
    // Valid unicode path with various scripts
    let path = PathBuf::from("文件_αβγ_файл.py");

    let result = analyzer.analyze_code("x = 1", path);

    // Should parse successfully
    assert!(result.parse_errors.is_empty());
}

/// Test that analyze_code handles paths with special characters.
#[test]
fn test_special_characters_path() {
    let analyzer = CytoScnPy::default();
    // Path with spaces, dashes, underscores, and dots
    let path = PathBuf::from("my file - (copy) [2024].py");

    let result = analyzer.analyze_code("def foo(): pass", path);

    // Should parse successfully
    assert!(result.parse_errors.is_empty());
}

/// Test that analyze_code handles empty path gracefully.
#[test]
fn test_empty_path() {
    let analyzer = CytoScnPy::default();
    let path = PathBuf::from("");

    // Should not panic on empty path
    let _result = analyzer.analyze_code("x = 1", path);
}

/// Test path with emoji characters (valid UTF-8 but unusual).
#[test]
fn test_emoji_path() {
    let analyzer = CytoScnPy::default();
    let path = PathBuf::from("🐍_script_🚀.py");

    let result = analyzer.analyze_code("print('emoji!')", path);

    // Should parse successfully
    assert!(result.parse_errors.is_empty());
}

/// Test very long path name.
#[test]
fn test_long_path() {
    let analyzer = CytoScnPy::default();
    // Create a reasonably long path name
    let long_name = "a".repeat(200) + ".py";
    let path = PathBuf::from(long_name);

    let result = analyzer.analyze_code("y = 2", path);

    // Should parse successfully
    assert!(result.parse_errors.is_empty());
}
