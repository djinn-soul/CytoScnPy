//! Unit tests for `loader`.

use super::*;

#[test]
fn test_load_python_whitelist() {
    let content = r"
# This is a comment
my_function  # Trailing comment
MyUnusedClass
_private_var

# Another comment
another_function
";
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
fn test_load_toml_whitelist() -> io::Result<()> {
    let content = r#"
[cytoscnpy]
whitelist = [
    { name = "my_function" },
    { name = "test_*", pattern = "wildcard" },
    { name = "api_.*", pattern = "regex" },
]
"#;
    let whitelist = load_toml_whitelist_from_str(content, Path::new("test.toml"))?;

    assert!(whitelist.is_whitelisted("my_function", None));
    assert!(whitelist.is_whitelisted("test_something", None));
    assert!(whitelist.is_whitelisted("api_handler", None));
    assert!(!whitelist.is_whitelisted("other_function", None));

    Ok(())
}

#[test]
fn test_empty_whitelist() {
    let content = "";
    let whitelist = load_python_whitelist_from_str(content);
    assert!(whitelist.is_empty());
}
