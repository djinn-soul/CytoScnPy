//! Tests for the commands module (`run_mi` multi flag functionality)
#![allow(clippy::unwrap_used)]
#![allow(clippy::needless_raw_string_hashes)]
#![allow(clippy::items_after_statements)]
#![allow(clippy::doc_markdown)]

use cytoscnpy::commands::run_mi;
use serde::Deserialize;
use std::fs;

#[derive(Deserialize, Debug)]
struct MiResult {
    #[allow(dead_code)]
    file: String,
    mi: f64,
    #[allow(dead_code)]
    rank: char,
}

#[test]
fn test_run_mi_multi_flag_integration() {
    // Setup temporary directory
    let temp_dir = tempfile::tempdir().unwrap();
    let file_path = temp_dir.path().join("test_multi_int.py");

    // Create the file content
    let mut content =
        String::from("\"\"\"\nThis is a docstring.\nIt spans multiple lines.\n\"\"\"\n");

    // 200 functions, each 2 lines of SLOC -> 400 SLOC.
    use std::fmt::Write;
    for i in 0..200 {
        write!(content, "\ndef f{i}():\n    return {i}\n").unwrap();
    }

    // 5 blocks of 3-line multi-strings = 15 lines.
    for _ in 0..5 {
        content.push_str("\n\"\"\"\nLine 1\n\"\"\"\n");
    }

    // Add complexity to ensure MI is not capped
    content.push_str(
        r#"
def complex_part(x):
    if x > 0:
        if x > 1:
            if x > 2:
                if x > 3:
                    if x > 4:
                        return x
    return 0
"#,
    );
    fs::write(&file_path, content).unwrap();

    // Run without multi flag
    let mut buffer_no_multi = Vec::new();
    run_mi(
        &file_path,
        cytoscnpy::commands::MiOptions {
            json: true,
            exclude: vec![],
            ignore: vec![],
            min_rank: None,
            max_rank: None,
            multi: false,
            show: true,
            average: false,
            fail_threshold: None,
            output_file: None,
        },
        &mut buffer_no_multi,
    )
    .unwrap();

    let output_no_multi = String::from_utf8(buffer_no_multi).unwrap();
    let results_no_multi: Vec<MiResult> = serde_json::from_str(&output_no_multi).unwrap();
    let mi_no_multi = results_no_multi[0].mi;

    // Run with multi flag
    let mut buffer_multi = Vec::new();
    run_mi(
        &file_path,
        cytoscnpy::commands::MiOptions {
            json: true,
            exclude: vec![],
            ignore: vec![],
            min_rank: None,
            max_rank: None,
            multi: true,
            show: true,
            average: false,
            fail_threshold: None,
            output_file: None,
        },
        &mut buffer_multi,
    )
    .unwrap();

    let output_multi = String::from_utf8(buffer_multi).unwrap();
    let results_multi: Vec<MiResult> = serde_json::from_str(&output_multi).unwrap();
    let mi_multi = results_multi[0].mi;

    println!("MI without multi: {mi_no_multi}");
    println!("MI with multi: {mi_multi}");

    // With multi flag, comments count should increase, and thus MI should increase
    assert!(
        mi_multi > mi_no_multi,
        "MI should increase when multi-line strings are counted as comments"
    );
}
