//! Whitelists must never be generated from incomplete reference information.

#![allow(clippy::unwrap_used)]

use cytoscnpy::entry_point::run_with_args_to;
use std::fs;
use tempfile::tempdir;

#[test]
fn whitelist_generation_rejects_parse_and_read_failures() {
    for invalid in [b"def broken(:\n".as_slice(), &[0xff, 0xfe]] {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("app.py"), "def unused():\n    return 1\n").unwrap();
        fs::write(dir.path().join("broken.py"), invalid).unwrap();
        let mut output = Vec::new();
        let error = run_with_args_to(
            vec![
                dir.path().to_string_lossy().into_owned(),
                "--make-whitelist".to_owned(),
            ],
            &mut output,
        )
        .unwrap_err();
        assert!(error
            .to_string()
            .contains("Cannot generate whitelist: scan incomplete"));
        assert!(!String::from_utf8(output).unwrap().contains("unused"));
    }
}

#[test]
fn complete_scan_still_generates_whitelist() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("app.py"), "def unused():\n    return 1\n").unwrap();
    let mut output = Vec::new();
    assert_eq!(
        run_with_args_to(
            vec![
                dir.path().to_string_lossy().into_owned(),
                "--make-whitelist".to_owned()
            ],
            &mut output,
        )
        .unwrap(),
        0
    );
    assert!(String::from_utf8(output).unwrap().contains("unused"));
}
