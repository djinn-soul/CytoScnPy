use super::{build_whitelist_matcher, clone_activation};

#[test]
fn html_only_runs_clones_without_enabling_clone_text() {
    assert_eq!(clone_activation(false, false, true), (true, false));
    assert_eq!(clone_activation(true, false, true), (true, true));
    assert_eq!(clone_activation(false, true, true), (true, true));
}

#[test]
fn build_whitelist_matcher_skips_when_no_entries() {
    let config = crate::config::Config::default();
    let matcher = build_whitelist_matcher(&config, &[]).expect("matcher build should succeed");
    assert!(matcher.is_none());
}

#[test]
fn build_whitelist_matcher_loads_external_whitelist_file() {
    let dir = tempfile::tempdir().expect("tempdir should be created");
    let whitelist_path = dir.path().join("whitelist.py");
    std::fs::write(&whitelist_path, "known_symbol\n").expect("whitelist file should be written");

    let config = crate::config::Config::default();
    let files = vec![whitelist_path];
    let matcher = build_whitelist_matcher(&config, &files)
        .expect("matcher build should succeed")
        .expect("matcher should be present");

    assert!(matcher.is_whitelisted("known_symbol", None));
}

#[test]
fn build_whitelist_matcher_rejects_invalid_config_pattern() {
    let mut config = crate::config::Config::default();
    config
        .cytoscnpy
        .whitelist
        .push(crate::config::WhitelistEntry {
            name: "keep_(me".to_owned(),
            pattern: Some(crate::config::WhitelistPattern::Regex),
            file: None,
            category: None,
        });
    let err = build_whitelist_matcher(&config, &[]).expect_err("invalid regex must fail");
    assert!(err.to_string().contains("keep_(me"), "{err}");
}

#[test]
fn build_whitelist_matcher_rejects_invalid_toml_whitelist_file() {
    let dir = tempfile::tempdir().expect("tempdir should be created");
    let path = dir.path().join("whitelist.toml");
    std::fs::write(
        &path,
        "whitelist = [{ name = \"bad[\", pattern = \"regex\" }]\n",
    )
    .expect("whitelist file should be written");
    let config = crate::config::Config::default();
    let err = build_whitelist_matcher(&config, &[path]).expect_err("invalid regex must fail");
    assert!(err.to_string().contains("bad["), "{err}");
}
