use crate::config::SecretsConfig;
use crate::utils::{get_line_suppression, LineIndex, Suppression};
use ruff_python_ast::Stmt;
use rustc_hash::FxHashSet;
use std::path::PathBuf;

use super::finding::SecretFinding;
use super::recognizers::SecretRecognizer;
use super::scoring::{ContextScorer, ScoringContext};
use super::{AstRecognizer, CustomRecognizer, EntropyRecognizer, RegexRecognizer};

/// Main secret scanner that orchestrates all recognizers.
pub struct SecretScanner {
    recognizers: Vec<Box<dyn SecretRecognizer>>,
    scorer: ContextScorer,
    min_score: u8,
    scan_comments: bool,
}

impl SecretScanner {
    /// Creates a new secret scanner from configuration.
    #[must_use]
    pub fn new(config: &SecretsConfig) -> Self {
        let mut recognizers: Vec<Box<dyn SecretRecognizer>> = Vec::new();
        recognizers.push(Box::new(RegexRecognizer));
        recognizers.push(Box::new(AstRecognizer::new(
            config.suspicious_names.clone(),
        )));

        if config.entropy_enabled {
            recognizers.push(Box::new(EntropyRecognizer::new(
                config.entropy_threshold,
                config.min_length,
            )));
        }

        if !config.patterns.is_empty() {
            recognizers.push(Box::new(CustomRecognizer::new(config)));
        }

        Self {
            recognizers,
            scorer: ContextScorer::new(),
            min_score: config.min_score,
            scan_comments: config.scan_comments,
        }
    }

    /// Scan content using all recognizers and apply scoring.
    #[must_use]
    pub fn scan(
        &self,
        content: &str,
        stmts: Option<&[Stmt]>,
        file_path: &PathBuf,
        line_index: &LineIndex,
        docstring_lines: Option<&FxHashSet<usize>>,
        is_test_file: bool,
    ) -> Vec<SecretFinding> {
        let mut all_findings = Vec::new();
        let lines: Vec<&str> = content.lines().collect();

        for recognizer in &self.recognizers {
            let text_findings = if stmts.is_some() {
                recognizer.scan_text(content, file_path)
            } else {
                recognizer.scan_text_fallback(content, file_path)
            };
            all_findings.extend(text_findings);

            if let Some(stmts) = stmts {
                let ast_findings = recognizer.scan_ast(stmts, file_path, line_index);
                all_findings.extend(ast_findings);
            }
        }

        let mut scored_findings = Vec::new();
        let mut seen_lines: FxHashSet<usize> = FxHashSet::default();

        for finding in all_findings {
            let line_idx = finding.line.saturating_sub(1);
            let line_content = lines.get(line_idx).unwrap_or(&"");

            let is_comment = line_content.trim().starts_with('#');
            if !self.scan_comments && is_comment {
                continue;
            }

            if let Some(suppression) = get_line_suppression(line_content) {
                match suppression {
                    Suppression::All => continue,
                    Suppression::Specific(rules) => {
                        if rules.contains(&finding.rule_id) {
                            continue;
                        }
                    }
                }
            }

            let is_docstring = docstring_lines.is_some_and(|lines| lines.contains(&finding.line));
            let context = ScoringContext {
                line_content,
                file_path,
                is_comment,
                is_docstring,
                is_test_file,
            };

            let confidence = self.scorer.score(finding.base_score, &context);
            if confidence < self.min_score {
                continue;
            }

            if seen_lines.contains(&finding.line) {
                if let Some(existing) = scored_findings
                    .iter_mut()
                    .find(|existing: &&mut SecretFinding| existing.line == finding.line)
                {
                    if confidence > existing.confidence {
                        existing.message = finding.message;
                        existing.rule_id = finding.rule_id;
                        existing.severity = finding.severity;
                        existing.matched_value = finding.matched_value;
                        existing.entropy = finding.entropy;
                        existing.confidence = confidence;
                    }
                }
                continue;
            }

            seen_lines.insert(finding.line);
            scored_findings.push(SecretFinding {
                message: finding.message,
                rule_id: finding.rule_id,
                category: "Secrets".to_owned(),
                file: file_path.clone(),
                line: finding.line,
                severity: finding.severity,
                matched_value: finding.matched_value,
                entropy: finding.entropy,
                confidence,
            });
        }

        scored_findings
    }
}
