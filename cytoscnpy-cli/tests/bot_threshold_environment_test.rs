//! Environment thresholds must fail visibly before scanning an empty project.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use std::process::Command;

#[test]
fn environment_thresholds_are_finite_percentages_and_respect_precedence() {
    let temp = tempfile::tempdir().unwrap();
    for value in ["NaN", "inf", "101", "-1", "invalid"] {
        let output = Command::new(env!("CARGO_BIN_EXE_cytoscnpy-cli"))
            .arg(temp.path())
            .arg("--json")
            .env("CYTOSCNPY_FAIL_THRESHOLD", value)
            .output()
            .unwrap();
        assert!(!output.status.success(), "{value}");
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("THRESHOLD")
                || String::from_utf8_lossy(&output.stderr).contains("fail_threshold")
        );
        let override_output = Command::new(env!("CARGO_BIN_EXE_cytoscnpy-cli"))
            .arg(temp.path())
            .args(["--json", "--fail-threshold", "100"])
            .env("CYTOSCNPY_FAIL_THRESHOLD", value)
            .output()
            .unwrap();
        assert!(
            override_output.status.success(),
            "{}",
            String::from_utf8_lossy(&override_output.stderr)
        );
    }
    for value in ["0", "100"] {
        let output = Command::new(env!("CARGO_BIN_EXE_cytoscnpy-cli"))
            .arg(temp.path())
            .arg("--json")
            .env("CYTOSCNPY_FAIL_THRESHOLD", value)
            .output()
            .unwrap();
        assert!(output.status.success());
    }
}
