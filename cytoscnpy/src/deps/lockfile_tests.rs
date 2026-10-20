//! Unit tests for `lockfile`.

use super::*;
use std::fs;
use tempfile::tempdir;

// Minimal valid uv.lock: version header + 3 packages with deps
fn uv_lock_sample() -> String {
    let mut s = String::new();
    s.push_str("version = 1\n");
    s.push_str("requires-python = \">=3.10\"\n\n");
    s.push_str("[[package]]\n");
    s.push_str("name = \"requests\"\n");
    s.push_str("version = \"2.31.0\"\n");
    s.push_str("source = { registry = \"https://pypi.org/simple\" }\n");
    s.push_str("dependencies = [\n");
    s.push_str("  { name = \"urllib3\" },\n");
    s.push_str("  { name = \"certifi\" },\n");
    s.push_str("]\n\n");
    s.push_str("[[package]]\n");
    s.push_str("name = \"urllib3\"\n");
    s.push_str("version = \"2.0.0\"\n");
    s.push_str("source = { registry = \"https://pypi.org/simple\" }\n\n");
    s.push_str("[[package]]\n");
    s.push_str("name = \"certifi\"\n");
    s.push_str("version = \"2023.7.22\"\n");
    s.push_str("source = { registry = \"https://pypi.org/simple\" }\n");
    s
}

// Real project uv.lock (tracked in git); fails loudly if it is missing.
#[test]
fn test_parse_uv_lock_real() {
    // Every `[[package]]` table in the real lockfile must be parsed
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("uv.lock");
    let content = fs::read_to_string(&path).expect("tracked uv.lock should exist at the repo root");
    let expected = content
        .lines()
        .filter(|line| line.trim() == "[[package]]")
        .count();
    let nodes = parse_uv_lock(&content);
    assert!(expected > 0, "real uv.lock should declare packages");
    assert_eq!(nodes.len(), expected, "every uv.lock package should parse");
}

#[test]
fn test_parse_uv_lock() {
    let s = uv_lock_sample();
    let nodes = parse_uv_lock(&s);
    assert_eq!(nodes.len(), 3, "expected 3 packages, got {}", nodes.len());
    let req = nodes.iter().find(|n| n.name == "requests").unwrap();
    assert!(req.deps.contains(&"urllib3".to_owned()));
    assert!(req.deps.contains(&"certifi".to_owned()));
}

#[test]
fn test_parse_poetry_lock() {
    let mut s = String::new();
    s.push_str("[[package]]\n");
    s.push_str("name = \"requests\"\n");
    s.push_str("version = \"2.31.0\"\n\n");
    s.push_str("[package.dependencies]\n");
    s.push_str("urllib3 = \">=1.21.1\"\n");
    s.push_str("certifi = \"*\"\n\n");
    s.push_str("[[package]]\n");
    s.push_str("name = \"urllib3\"\n");
    s.push_str("version = \"2.0.0\"\n\n");
    s.push_str("[[package]]\n");
    s.push_str("name = \"certifi\"\n");
    s.push_str("version = \"2023.7.22\"\n");

    let nodes = parse_poetry_lock(&s);
    assert_eq!(nodes.len(), 3, "expected 3 packages, got {}", nodes.len());
    let req = nodes.iter().find(|n| n.name == "requests").unwrap();
    assert!(req.deps.contains(&"urllib3".to_owned()));
}

#[test]
fn test_transitive_deps() {
    let s = uv_lock_sample();
    let nodes = parse_uv_lock(&s);
    let graph = LockfileGraph::from_nodes(&nodes);
    let transitive = graph.transitive_deps("requests");
    assert!(transitive.contains(&"urllib3".to_owned()));
    assert!(transitive.contains(&"certifi".to_owned()));
    assert!(!transitive.contains(&"requests".to_owned()));
}

#[test]
fn test_reverse_map() {
    let s = uv_lock_sample();
    let nodes = parse_uv_lock(&s);
    let graph = LockfileGraph::from_nodes(&nodes);
    let parents = graph.reverse.get("urllib3").unwrap();
    assert!(parents.contains(&"requests".to_owned()));
}

#[test]
fn test_load_lockfile_graph_at_explicit_path() {
    let tmp = tempdir().unwrap();

    let uv = tmp.path().join("uv.lock");
    fs::write(&uv, uv_lock_sample()).unwrap();
    let graph = load_lockfile_graph_at(&uv).expect("uv.lock parses");
    assert!(graph.forward.contains_key("requests"));

    // Recognized by file name, not by content.
    let unknown = tmp.path().join("Pipfile.lock");
    fs::write(&unknown, uv_lock_sample()).unwrap();
    assert!(load_lockfile_graph_at(&unknown).is_none());

    // A lockfile with no packages yields no graph.
    let empty = tmp.path().join("empty").join("poetry.lock");
    fs::create_dir_all(empty.parent().unwrap()).unwrap();
    fs::write(&empty, "").unwrap();
    assert!(load_lockfile_graph_at(&empty).is_none());

    assert!(load_lockfile_graph_at(&tmp.path().join("absent.lock")).is_none());
}

#[test]
fn test_load_lockfile_graph_prefers_uv_then_poetry() {
    let tmp = tempdir().unwrap();
    assert!(load_lockfile_graph(tmp.path()).is_none());

    fs::write(
        tmp.path().join("poetry.lock"),
        "[[package]]\nname = \"certifi\"\nversion = \"1.0\"\n",
    )
    .unwrap();
    let graph =
        load_lockfile_graph(tmp.path()).expect("poetry.lock is used when uv.lock is absent");
    assert!(graph.forward.contains_key("certifi"));

    fs::write(tmp.path().join("uv.lock"), uv_lock_sample()).unwrap();
    let graph = load_lockfile_graph(tmp.path()).expect("uv.lock wins");
    assert!(graph.forward.contains_key("requests"));
}

#[test]
fn test_load_lockfile_graph_from_temp_dir() {
    let tmp = tempdir().unwrap();
    fs::write(tmp.path().join("uv.lock"), uv_lock_sample()).unwrap();
    let graph = load_lockfile_graph(tmp.path());
    assert!(graph.is_some());
    let graph = graph.unwrap();
    assert!(graph.forward.contains_key("requests"));
}
