//! Integration tests for the settings module.
//!
//! These tests verify the global initialization, reset, and access patterns
//! for the application configuration stored in the settings singleton.

use cytoscnpy::config::Config;
use cytoscnpy::settings::{self, SettingsError};

fn prepare() {
    settings::reset_for_tests();
}

#[test]
fn initialize_exposes_config() {
    prepare();
    let mut config = Config::default();
    config.cytoscnpy.confidence = Some(42);

    settings::initialize(config).unwrap();

    let stored = settings::config().unwrap();
    assert_eq!(stored.cytoscnpy.confidence, Some(42));

    settings::reset_for_tests();
}

#[test]
fn initialize_twice_errors() {
    prepare();
    settings::initialize(Config::default()).unwrap();

    let err = settings::initialize(Config::default()).unwrap_err();
    assert_eq!(err, SettingsError::AlreadyInitialized);

    settings::reset_for_tests();
}

#[test]
fn reset_allows_reinitialization() {
    prepare();
    settings::initialize(Config::default()).unwrap();
    settings::reset_for_tests();

    assert!(!settings::is_initialized());

    settings::initialize(Config::default()).unwrap();
    settings::reset_for_tests();

    assert!(!settings::is_initialized());
}
