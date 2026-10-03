//! HTML paths must preserve separate sources and all links into their file views.
#![cfg(feature = "html_report")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use cytoscnpy::{
    analyzer::CytoScnPy,
    commands::{run_clones, CloneOptions},
    report::generator::generate_report,
};
use regex::Regex;
use std::{collections::BTreeSet, fs, path::Path};

fn source(marker: &str) -> String {
    format!("print('{marker}')\ndef exact_copy(value):\n    first = 1\n    second = 2\n    total = first + second\n    return value * total\n")
}

fn links(page: &str) -> Vec<String> {
    Regex::new(r#"(?:href|data-link|data-related-link)="(files/[^"]+\.html)(?:#[^"]*)?""#)
        .unwrap()
        .captures_iter(page)
        .map(|capture| capture[1].to_owned())
        .collect()
}

fn assert_source_links(output: &Path, page: &str) -> BTreeSet<String> {
    let links: BTreeSet<_> = links(page).into_iter().collect();
    for link in &links {
        assert!(
            output.join(link).is_file(),
            "Missing linked file view: {link}"
        );
    }
    links
}

#[test]
fn colliding_paths_have_distinct_views_and_consistent_issue_and_clone_links() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join("a")).unwrap();
    fs::write(dir.path().join("a/b.py"), source("FIRST_FILE")).unwrap();
    fs::write(dir.path().join("a_b.py"), source("SECOND_FILE")).unwrap();
    let mut analyzer = CytoScnPy::default().with_quality(true);
    let mut result = analyzer.analyze(dir.path());
    let options = CloneOptions {
        similarity: 1.0,
        json: true,
        ..Default::default()
    };
    let (_, findings) = run_clones(&[dir.path().to_path_buf()], &options, &mut Vec::new()).unwrap();
    assert!(!findings.is_empty());
    result.clones = findings;
    let output = dir.path().join("report");
    generate_report(&result, dir.path(), &output).unwrap();
    let metrics = fs::read_to_string(output.join("files.html")).unwrap();
    let metrics_links = assert_source_links(&output, &metrics);
    assert_eq!(metrics_links.len(), 2);
    let views: Vec<_> = fs::read_dir(output.join("files"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    assert_eq!(views.len(), 2);
    let contents: Vec<_> = views
        .iter()
        .map(|path| fs::read_to_string(path).unwrap())
        .collect();
    assert_eq!(
        contents
            .iter()
            .filter(|content| content.contains("FIRST_FILE"))
            .count(),
        1
    );
    assert_eq!(
        contents
            .iter()
            .filter(|content| content.contains("SECOND_FILE"))
            .count(),
        1
    );
    let issues = fs::read_to_string(output.join("issues.html")).unwrap();
    assert_eq!(assert_source_links(&output, &issues), metrics_links);
    let clones = fs::read_to_string(output.join("clones.html")).unwrap();
    assert_eq!(assert_source_links(&output, &clones), metrics_links);
    // Registry identifiers remain consistent when analysis result order changes.
    result.file_metrics.reverse();
    result.clones.reverse();
    let second = dir.path().join("second-report");
    generate_report(&result, dir.path(), &second).unwrap();
    let second_page = fs::read_to_string(second.join("files.html")).unwrap();
    assert_eq!(assert_source_links(&second, &second_page), metrics_links);
}
