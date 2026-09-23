//! Integration tests for Python source language and configuration format detection.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use tempfile::TempDir;

use cytoscnpy::doctor::{scan_repo_structure, SupportedLanguage};

#[test]
fn test_supported_language_classification() {
    // Only Python is supported as a coding language
    assert!(
        SupportedLanguage::Python.is_source_code(),
        "Expected Python to be source code"
    );
    assert!(
        !SupportedLanguage::Python.is_config_or_document(),
        "Expected Python not to be config/doc"
    );
    assert_eq!(SupportedLanguage::Python.label(), "Python");

    let configs_docs = [
        SupportedLanguage::Shell,
        SupportedLanguage::Toml,
        SupportedLanguage::Yaml,
        SupportedLanguage::Json,
        SupportedLanguage::Markdown,
        SupportedLanguage::Html,
        SupportedLanguage::Css,
        SupportedLanguage::Docker,
        SupportedLanguage::Xml,
        SupportedLanguage::Ini,
    ];

    for fmt in configs_docs {
        assert!(
            fmt.is_config_or_document(),
            "Expected {fmt:?} to be config or document"
        );
        assert!(
            !fmt.is_source_code(),
            "Expected {fmt:?} not to be source code"
        );
        assert!(!fmt.label().is_empty());
    }
}

#[test]
fn test_python_only_repo_structure_scanning() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    // Create Python source files
    fs::write(root.join("main.py"), "print('hello')\n").unwrap();
    fs::write(root.join("types.pyi"), "x: int\n").unwrap();

    // Create configuration, document, and build files
    fs::write(root.join("run.sh"), "#!/bin/sh\necho ok\n").unwrap();
    fs::write(root.join("Makefile"), "all:\n\techo done\n").unwrap();
    fs::write(root.join("Dockerfile"), "FROM alpine:latest\n").unwrap();
    fs::write(root.join("pyproject.toml"), "[project]\nname = \"pkg\"\n").unwrap();
    fs::write(root.join("config.yaml"), "key: value\n").unwrap();
    fs::write(root.join("data.json"), "{\"status\": \"ok\"}\n").unwrap();
    fs::write(root.join("README.md"), "# Project Docs\n").unwrap();
    fs::write(root.join("index.html"), "<html></html>\n").unwrap();
    fs::write(root.join("style.css"), "body { margin: 0; }\n").unwrap();
    fs::write(root.join("feed.xml"), "<root><item/></root>\n").unwrap();
    fs::write(root.join(".env"), "PORT=8080\n").unwrap();

    // Non-Python languages should NOT be detected
    fs::write(root.join("lib.rs"), "pub fn add() {}\n").unwrap();
    fs::write(root.join("server.go"), "package main\n").unwrap();
    fs::write(root.join("app.ts"), "const x: number = 42;\n").unwrap();
    fs::write(root.join("script.rb"), "puts 'ruby'\n").unwrap();
    fs::write(root.join("index.php"), "<?php echo 'hi'; ?>\n").unwrap();
    fs::write(root.join("core.c"), "int main() { return 0; }\n").unwrap();
    fs::write(root.join("query.sql"), "SELECT 1;\n").unwrap();
    fs::write(root.join("ignored.unknownext"), "binary or unknown\n").unwrap();

    let stats = scan_repo_structure(root, &[]);

    let detected: std::collections::HashSet<String> =
        stats.languages.into_iter().map(|l| l.language).collect();

    // Python source code detected
    assert!(detected.contains("Python"), "Missing Python");

    // Config, document, and build formats detected
    assert!(detected.contains("Shell"), "Missing Shell");
    assert!(detected.contains("Docker"), "Missing Docker");
    assert!(detected.contains("TOML"), "Missing TOML");
    assert!(detected.contains("YAML"), "Missing YAML");
    assert!(detected.contains("JSON"), "Missing JSON");
    assert!(detected.contains("Markdown"), "Missing Markdown");
    assert!(detected.contains("HTML"), "Missing HTML");
    assert!(detected.contains("CSS"), "Missing CSS");
    assert!(detected.contains("XML"), "Missing XML");
    assert!(detected.contains("INI"), "Missing INI");

    // Other programming languages must NOT be detected
    assert!(!detected.contains("Rust"), "Rust must not be detected");
    assert!(!detected.contains("Go"), "Go must not be detected");
    assert!(
        !detected.contains("TypeScript"),
        "TypeScript must not be detected"
    );
    assert!(!detected.contains("Ruby"), "Ruby must not be detected");
    assert!(!detected.contains("PHP"), "PHP must not be detected");
    assert!(!detected.contains("C"), "C must not be detected");
    assert!(!detected.contains("SQL"), "SQL must not be detected");
    assert!(!detected.contains("unknownext"));
}
