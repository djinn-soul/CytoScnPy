//! Server settings, named like the VS Code extension's `cytoscnpy.*`
//! settings without the prefix.
//!
//! Clients pass them as initialization options or as workspace
//! configuration, either directly or under a `cytoscnpy` key:
//!
//! ```json
//! { "enableSecretsScan": true, "confidenceThreshold": 80, "args": ["--quality"] }
//! ```

use serde_json::Value;

/// How to analyze, from the client's settings.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Settings {
    /// Analyze each file alone instead of its whole project
    /// (`"analysisMode": "file"`).
    pub file_mode: bool,
    /// CLI flags added to every analysis.
    pub args: Vec<String>,
}

impl Settings {
    /// Reads settings from `value`, ignoring unknown keys and wrong types.
    pub(crate) fn from_value(value: Option<&Value>) -> Self {
        let Some(value) = value else {
            return Self::default();
        };
        let value = value
            .get("cytoscnpy")
            .filter(|nested| nested.is_object())
            .unwrap_or(value);
        let bool_of = |key: &str| value.get(key).and_then(Value::as_bool);
        let number_of = |key: &str| value.get(key).and_then(Value::as_f64);
        let strings_of = |key: &str| -> Vec<String> {
            value
                .get(key)
                .and_then(Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|item| item.as_str().map(str::to_owned))
                        .collect()
                })
                .unwrap_or_default()
        };

        // The same flags, in the same order, as the VS Code extension's
        // `buildAnalyzerArgs`. An explicit false overrides the project.
        let mut args = Vec::new();
        for (key, scan) in [
            ("enableSecretsScan", "secrets"),
            ("enableDangerScan", "danger"),
            ("enableQualityScan", "quality"),
            ("enableCloneScan", "clones"),
        ] {
            if let Some(enabled) = bool_of(key) {
                args.push(if enabled {
                    format!("--{scan}")
                } else {
                    format!("--no-{scan}")
                });
            }
        }
        if let Some(confidence) = number_of("confidenceThreshold") {
            args.extend(["--confidence".to_owned(), format_number(confidence)]);
        }
        for (key, flag) in [
            ("excludeFolders", "--exclude-folders"),
            ("includeFolders", "--include-folders"),
        ] {
            for folder in strings_of(key) {
                args.extend([flag.to_owned(), folder]);
            }
        }
        for (key, files) in [("includeTests", "tests"), ("includeIpynb", "ipynb")] {
            if let Some(included) = bool_of(key) {
                args.push(if included {
                    format!("--include-{files}")
                } else {
                    format!("--no-include-{files}")
                });
            }
        }
        // Thresholds enable quality analysis on their own, unless it is
        // disabled.
        if bool_of("enableQualityScan") != Some(false) {
            for (key, flag) in [
                ("maxComplexity", "--max-complexity"),
                ("minMaintainabilityIndex", "--min-mi"),
                ("maxNesting", "--max-nesting"),
                ("maxArguments", "--max-args"),
                ("maxLines", "--max-lines"),
            ] {
                if let Some(limit) = number_of(key) {
                    args.extend([flag.to_owned(), format_number(limit)]);
                }
            }
        }
        args.extend(strings_of("args"));

        Self {
            file_mode: value.get("analysisMode").and_then(Value::as_str) == Some("file"),
            args,
        }
    }

    /// `self`, with what `other` sets taking precedence: workspace
    /// configuration over initialization options.
    pub(crate) fn merged(&self, other: &Self) -> Self {
        Self {
            file_mode: self.file_mode || other.file_mode,
            args: self.args.iter().chain(&other.args).cloned().collect(),
        }
    }
}

/// `80` rather than `80.0`, as JSON settings hold numbers as floats.
fn format_number(number: f64) -> String {
    if number.fract() == 0.0 && number.abs() < 1e15 {
        format!("{number:.0}")
    } else {
        number.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::Settings;
    use serde_json::json;

    #[test]
    fn maps_vscode_settings_to_cli_flags() {
        let settings = Settings::from_value(Some(&json!({
            "enableSecretsScan": true,
            "enableDangerScan": false,
            "confidenceThreshold": 80,
            "excludeFolders": ["build", "dist"],
            "includeTests": true,
            "maxComplexity": 12,
            "minMaintainabilityIndex": 40.5,
            "analysisMode": "file",
            "args": ["--verbose"],
        })));
        assert!(settings.file_mode);
        assert_eq!(
            settings.args,
            [
                "--secrets",
                "--no-danger",
                "--confidence",
                "80",
                "--exclude-folders",
                "build",
                "--exclude-folders",
                "dist",
                "--include-tests",
                "--max-complexity",
                "12",
                "--min-mi",
                "40.5",
                "--verbose",
            ]
        );
    }

    #[test]
    fn reads_settings_under_a_cytoscnpy_key_and_skips_disabled_thresholds() {
        let settings = Settings::from_value(Some(&json!({
            "cytoscnpy": {"enableQualityScan": false, "maxLines": 50}
        })));
        assert!(!settings.file_mode);
        assert_eq!(settings.args, ["--no-quality"]);
        assert_eq!(Settings::from_value(None), Settings::default());
    }
}
