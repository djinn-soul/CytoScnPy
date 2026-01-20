//! Tests for suppression functionality (noqa comments).

#[cfg(test)]
mod tests {
    use cytoscnpy::utils::{get_ignored_lines, is_line_suppressed, Suppression};

    #[test]
    fn test_noqa_csp_suppression() {
        let source = "os.system(cmd) # noqa: CSP\n";
        let ignored = get_ignored_lines(source);

        assert!(ignored.contains_key(&1), "Line 1 should have suppression");
        assert!(matches!(ignored.get(&1), Some(Suppression::All)));

        // Standard rule ID should be suppressed
        assert!(is_line_suppressed(&ignored, 1, "CSP-D003"));

        // Taint rule ID (if generic CSP is used)
        assert!(is_line_suppressed(&ignored, 1, "taint-commandinjection"));
    }

    #[test]
    fn test_noqa_specific_suppression() {
        let source = "os.system(cmd) # noqa: CSP-D003\n";
        let ignored = get_ignored_lines(source);

        assert!(ignored.contains_key(&1));

        // Specific rule ID should be suppressed
        assert!(is_line_suppressed(&ignored, 1, "CSP-D003"));

        // OTHER rule IDs should NOT be suppressed
        assert!(!is_line_suppressed(&ignored, 1, "CSP-D001"));
        assert!(!is_line_suppressed(&ignored, 1, "taint-commandinjection"));
    }

    #[test]
    fn test_analyzer_respects_suppression() {
        use cytoscnpy::analyzer::CytoScnPy;
        let source = "import os\nos.system('ls') # noqa: CSP\n";
        let analyzer = CytoScnPy {
            enable_danger: true,
            ..CytoScnPy::default()
        };

        let result = analyzer.analyze_code(source, &std::path::PathBuf::from("test.py"));

        // Finding on line 2 should be suppressed
        assert_eq!(
            result.danger.len(),
            0,
            "Finding should have been suppressed by # noqa: CSP"
        );
    }

    #[test]
    fn test_real_file_suppression() {
        use cytoscnpy::analyzer::CytoScnPy;
        use std::path::PathBuf;

        let file_path = PathBuf::from("tests/python_files/suppression_case.py");
        let mut analyzer = CytoScnPy {
            enable_danger: true,
            ..CytoScnPy::default()
        };

        let result = analyzer.analyze_paths(&[file_path]);

        println!("--- Danger Findings: {} ---", result.danger.len());
        for f in &result.danger {
            println!(
                "Danger finding on line {}: {} (ID: {})",
                f.line, f.message, f.rule_id
            );
        }
        println!("--- Taint Findings: {} ---", result.taint_findings.len());
        for f in &result.taint_findings {
            println!(
                "Taint finding on line {}: {} (Type: {:?})",
                f.sink_line, f.sink, f.vuln_type
            );
        }

        let danger_lines: Vec<usize> = result.danger.iter().map(|f| f.line).collect();
        let taint_lines: Vec<usize> = result.taint_findings.iter().map(|f| f.sink_line).collect();

        // Line 7: Should have both
        assert!(
            danger_lines.contains(&7),
            "Line 7 should have a danger finding"
        );
        // assert!(taint_lines.contains(&7), "Line 7 should have a taint finding");

        // Line 12: Suppressed by generic noqa
        assert!(
            !danger_lines.contains(&12),
            "Line 12 should be suppressed by generic noqa"
        );
        assert!(
            !taint_lines.contains(&12),
            "Line 12 taint should be suppressed"
        );

        // Line 18: Suppressed by specific CSP-D003
        assert!(
            !danger_lines.contains(&18),
            "Line 18 should be suppressed by specific code"
        );
        assert!(
            !taint_lines.contains(&18),
            "Line 18 taint should be suppressed"
        );

        // Line 23: NOT suppressed by CSP-X999
        assert!(
            danger_lines.contains(&23),
            "Line 23 should NOT be suppressed by mismatching code"
        );
    }
}
