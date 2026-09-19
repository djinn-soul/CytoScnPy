use serde::{Deserialize, Serialize};

/// Summary of repository structure, language breakdown, test/source counts, and configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RepoSummary {
    /// Total scanned files across all recognized extensions.
    pub total_files: usize,
    /// Total lines across all files.
    pub total_lines: usize,
    /// Total size in bytes.
    pub total_bytes: u64,
    /// Average lines per file.
    pub avg_file_lines: usize,
    /// Maximum file lines in largest file.
    pub max_file_lines: usize,
    /// Path of the largest file.
    pub largest_file_path: String,
    /// Number of production source files.
    pub source_files: usize,
    /// Number of source code lines.
    pub source_lines: usize,
    /// Number of test files.
    pub test_files: usize,
    /// Number of test code lines.
    pub test_lines: usize,
    /// Ratio of test lines to source lines (e.g. 0.35 = 35%).
    pub test_to_source_ratio: f64,
    /// Breakdown of files, lines, and bytes per language.
    pub languages: Vec<LanguageBreakdown>,
    /// Detected repository tooling and configuration files.
    pub detected_configs: Vec<String>,
}

/// Breakdown of code volume for a specific language.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LanguageBreakdown {
    /// Language name (e.g. "Python", "Rust", "TOML").
    pub name: String,
    /// Number of files.
    pub files: usize,
    /// Number of lines of code.
    pub lines: usize,
    /// Size in bytes.
    pub bytes: u64,
}

impl RepoSummary {
    /// Create a summary from doctor inspection results.
    #[must_use]
    pub fn from_doctor(doc: &crate::doctor::DoctorResult) -> Self {
        let mut configs = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for c in &doc.configs {
            let label = format!("{}: {}", c.category, c.name);
            if seen.insert(label.clone()) {
                configs.push(label);
            }
        }

        let languages = doc
            .structure
            .languages
            .iter()
            .map(|l| LanguageBreakdown {
                name: l.language.clone(),
                files: l.file_count,
                lines: l.line_count,
                bytes: l.byte_count,
            })
            .collect();

        Self {
            total_files: doc.structure.total_files,
            total_lines: doc.structure.total_lines,
            total_bytes: doc.structure.total_bytes,
            avg_file_lines: doc.structure.avg_file_lines,
            max_file_lines: doc.structure.largest_file_lines,
            largest_file_path: doc.structure.largest_file_path.clone(),
            source_files: doc.structure.source_files,
            source_lines: doc.structure.source_lines,
            test_files: doc.structure.test_files,
            test_lines: doc.structure.test_lines,
            test_to_source_ratio: doc.structure.test_to_source_ratio,
            languages,
            detected_configs: configs,
        }
    }

    /// Format summary section for terminal reporting.
    #[must_use]
    pub fn format_terminal_summary(&self) -> String {
        let mut out = String::new();
        out.push_str("Codebase Summary:\n");
        out.push_str(&format!(
            "  Files: {:<6} Lines: {:<8} Bytes: {:<10} Max File Lines: {}\n",
            self.total_files, self.total_lines, self.total_bytes, self.max_file_lines
        ));
        out.push_str(&format!(
            "  Source Files: {:<6} ({} lines)   Test Files: {:<6} ({} lines)   Test Ratio: {:.1}%\n",
            self.source_files,
            self.source_lines,
            self.test_files,
            self.test_lines,
            self.test_to_source_ratio * 100.0
        ));

        if !self.languages.is_empty() {
            let lang_parts: Vec<String> = self
                .languages
                .iter()
                .take(6)
                .map(|l| format!("{} ({} files, {} lines)", l.name, l.files, l.lines))
                .collect();
            out.push_str(&format!("  Languages: {}\n", lang_parts.join(", ")));
        }

        if !self.detected_configs.is_empty() {
            let config_parts: Vec<String> = self.detected_configs.iter().take(6).cloned().collect();
            out.push_str(&format!(
                "  Tooling Detected: {}\n",
                config_parts.join(", ")
            ));
        }
        out.push('\n');
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_repo_summary_terminal_format() {
        let summary = RepoSummary {
            total_files: 42,
            total_lines: 5400,
            total_bytes: 185_000,
            avg_file_lines: 128,
            max_file_lines: 450,
            largest_file_path: "src/engine.py".to_owned(),
            source_files: 30,
            source_lines: 4000,
            test_files: 12,
            test_lines: 1400,
            test_to_source_ratio: 0.35,
            languages: vec![
                LanguageBreakdown {
                    name: "Python".to_owned(),
                    files: 35,
                    lines: 4800,
                    bytes: 160_000,
                },
                LanguageBreakdown {
                    name: "TOML".to_owned(),
                    files: 2,
                    lines: 100,
                    bytes: 3_000,
                },
            ],
            detected_configs: vec![
                "Linter: Ruff".to_owned(),
                "Formatter: Black".to_owned(),
                "Lockfile: poetry.lock".to_owned(),
            ],
        };

        let formatted = summary.format_terminal_summary();
        assert!(formatted.contains("Codebase Summary:"));
        assert!(formatted.contains("Files: 42"));
        assert!(formatted.contains("Lines: 5400"));
        assert!(formatted.contains("Source Files: 30"));
        assert!(formatted.contains("Test Files: 12"));
        assert!(formatted.contains("Test Ratio: 35.0%"));
        assert!(formatted.contains("Python (35 files, 4800 lines)"));
        assert!(formatted.contains("Tooling Detected: Linter: Ruff, Formatter: Black"));
    }

    #[test]
    fn test_repo_summary_serialization() {
        let summary = RepoSummary {
            total_files: 10,
            total_lines: 1000,
            total_bytes: 35000,
            avg_file_lines: 100,
            max_file_lines: 200,
            largest_file_path: "main.py".to_owned(),
            source_files: 8,
            source_lines: 800,
            test_files: 2,
            test_lines: 200,
            test_to_source_ratio: 0.25,
            languages: vec![LanguageBreakdown {
                name: "Python".to_owned(),
                files: 10,
                lines: 1000,
                bytes: 35000,
            }],
            detected_configs: vec!["CI: GitHub Actions".to_owned()],
        };

        let json = serde_json::to_string(&summary).unwrap();
        assert!(json.contains("\"total_files\":10"));
        assert!(json.contains("\"test_to_source_ratio\":0.25"));
        assert!(json.contains("\"Python\""));
    }
}
