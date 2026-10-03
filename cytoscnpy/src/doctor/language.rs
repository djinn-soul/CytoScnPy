//! Python source language and configuration/document format detection.
//!
//! CytoScnPy focuses specifically on Python codebases while recognizing common
//! repository configuration, build automation, and documentation formats.

use serde::{Deserialize, Serialize};
use std::path::Path;

/// Supported source language (Python) or common configuration/document format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SupportedLanguage {
    /// Python source files (.py, .pyi).
    Python,
    /// Shell scripts and Makefiles (.sh, .bash, .zsh, Makefile).
    Shell,
    /// TOML configuration files (.toml).
    Toml,
    /// YAML configuration files (.yaml, .yml).
    Yaml,
    /// JSON data/configuration files (.json).
    Json,
    /// Markdown documentation files (.md, .markdown, .mdx).
    Markdown,
    /// HTML template and document files (.html, .htm).
    Html,
    /// CSS/SCSS/LESS stylesheets (.css, .scss, .less).
    Css,
    /// Docker and container definitions (Dockerfile, Containerfile).
    Docker,
    /// XML and SVG markup files (.xml, .svg).
    Xml,
    /// INI and environment configuration files (.ini, .cfg, .conf, .env).
    Ini,
}

impl SupportedLanguage {
    /// Canonical human-readable label for this language or format.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Python => "Python",
            Self::Shell => "Shell",
            Self::Toml => "TOML",
            Self::Yaml => "YAML",
            Self::Json => "JSON",
            Self::Markdown => "Markdown",
            Self::Html => "HTML",
            Self::Css => "CSS",
            Self::Docker => "Docker",
            Self::Xml => "XML",
            Self::Ini => "INI",
        }
    }

    /// Whether this format represents the primary programming source language (Python).
    #[must_use]
    pub const fn is_source_code(self) -> bool {
        matches!(self, Self::Python)
    }

    /// Whether this format is a configuration, data, or documentation format.
    #[must_use]
    pub const fn is_config_or_document(self) -> bool {
        !self.is_source_code()
    }

    /// Detect language from path.
    #[must_use]
    pub fn detect(path: &Path) -> Option<Self> {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_lowercase();
        detect_from_ext_and_name(&ext, &file_name)
    }
}

/// Detect language or format from lowercase file extension and lowercase filename.
///
/// Restricts programming language detection strictly to Python, while recognizing
/// repository configuration, documentation, and build automation formats.
#[must_use]
pub fn detect_from_ext_and_name(ext: &str, file_name: &str) -> Option<SupportedLanguage> {
    match ext {
        "py" | "pyi" => Some(SupportedLanguage::Python),
        "sh" | "bash" | "zsh" => Some(SupportedLanguage::Shell),
        "toml" => Some(SupportedLanguage::Toml),
        "yaml" | "yml" => Some(SupportedLanguage::Yaml),
        "json" => Some(SupportedLanguage::Json),
        "md" | "markdown" | "mdx" => Some(SupportedLanguage::Markdown),
        "html" | "htm" => Some(SupportedLanguage::Html),
        "css" | "scss" | "less" => Some(SupportedLanguage::Css),
        "xml" | "svg" => Some(SupportedLanguage::Xml),
        "ini" | "cfg" | "conf" | "env" => Some(SupportedLanguage::Ini),
        _ => match file_name {
            "makefile" | "gnumakefile" => Some(SupportedLanguage::Shell),
            "dockerfile" | "containerfile" => Some(SupportedLanguage::Docker),
            _ if file_name == ".env" || file_name.starts_with(".env.") => {
                Some(SupportedLanguage::Ini)
            }
            _ => None,
        },
    }
}

/// Detect language label from lowercase extension and filename.
#[must_use]
pub fn detect_language(ext: &str, file_name: &str) -> Option<&'static str> {
    detect_from_ext_and_name(ext, file_name).map(SupportedLanguage::label)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_detect_python_only_source_language() {
        assert_eq!(
            SupportedLanguage::detect(&PathBuf::from("main.py")),
            Some(SupportedLanguage::Python)
        );
        assert_eq!(
            SupportedLanguage::detect(&PathBuf::from("stub.pyi")),
            Some(SupportedLanguage::Python)
        );

        // Non-Python coding languages must NOT be detected as supported languages
        let non_python_sources = [
            "lib.rs",
            "app.ts",
            "app.tsx",
            "index.js",
            "index.jsx",
            "server.go",
            "App.java",
            "main.c",
            "engine.cpp",
            "script.rb",
            "Gemfile",
            "index.php",
            "Program.cs",
            "main.swift",
            "Main.kt",
            "App.scala",
            "init.lua",
            "main.zig",
            "main.dart",
            "lib.ex",
            "Main.hs",
            "main.ml",
            "schema.sql",
        ];
        for path in non_python_sources {
            assert_eq!(
                SupportedLanguage::detect(&PathBuf::from(path)),
                None,
                "Non-Python source {path} should not be detected as supported"
            );
        }
    }

    #[test]
    fn test_detect_config_and_docs() {
        let cases = [
            ("Cargo.toml", SupportedLanguage::Toml),
            ("ci.yml", SupportedLanguage::Yaml),
            ("ci.yaml", SupportedLanguage::Yaml),
            ("package.json", SupportedLanguage::Json),
            ("README.md", SupportedLanguage::Markdown),
            ("index.html", SupportedLanguage::Html),
            ("style.css", SupportedLanguage::Css),
            ("Dockerfile", SupportedLanguage::Docker),
            ("config.xml", SupportedLanguage::Xml),
            (".env", SupportedLanguage::Ini),
            ("settings.ini", SupportedLanguage::Ini),
            ("deploy.sh", SupportedLanguage::Shell),
            ("Makefile", SupportedLanguage::Shell),
        ];
        for (path, expected) in cases {
            assert_eq!(
                SupportedLanguage::detect(&PathBuf::from(path)),
                Some(expected)
            );
        }
    }

    #[test]
    fn test_language_predicates() {
        assert!(SupportedLanguage::Python.is_source_code());
        assert!(!SupportedLanguage::Python.is_config_or_document());
        assert!(!SupportedLanguage::Markdown.is_source_code());
        assert!(SupportedLanguage::Markdown.is_config_or_document());
        assert!(SupportedLanguage::Toml.is_config_or_document());
        assert!(SupportedLanguage::Shell.is_config_or_document());
    }
}
