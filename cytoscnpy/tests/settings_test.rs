//! Integration tests for the settings module.
//!
//! These tests verify the global initialization, reset, and access patterns
//! for the application configuration stored in the settings singleton.
//!
//! IMPORTANT: all tests in this file share the same process-global SETTINGS
//! singleton.  They must run sequentially; `TEST_SERIAL` is a module-level
//! mutex that serializes them.  Each test holds the guard for its entire
//! duration so that no two tests can touch the singleton at the same time.
#![allow(clippy::unwrap_used)]

use cytoscnpy::config::Config;
use cytoscnpy::settings::{self, SettingsError};
use std::sync::Mutex;

static TEST_SERIAL: Mutex<()> = Mutex::new(());

/// Lock `TEST_SERIAL` and reset the singleton.  The returned guard must be
/// kept alive (bound to `_guard`) for the duration of the test.
fn prepare() -> std::sync::MutexGuard<'static, ()> {
    let guard = TEST_SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    settings::reset_for_tests();
    guard
}

#[test]
fn initialize_exposes_config() {
    let _guard = prepare();
    let mut config = Config::default();
    config.cytoscnpy.confidence = Some(42);

    settings::initialize(config).unwrap();

    let stored = settings::config().unwrap();
    assert_eq!(stored.cytoscnpy.confidence, Some(42));

    settings::reset_for_tests();
}

#[test]
fn initialize_twice_errors() {
    let _guard = prepare();
    settings::initialize(Config::default()).unwrap();

    let err = settings::initialize(Config::default()).unwrap_err();
    assert_eq!(err, SettingsError::AlreadyInitialized);

    settings::reset_for_tests();
}

#[test]
fn reset_allows_reinitialization() {
    let _guard = prepare();
    settings::initialize(Config::default()).unwrap();
    settings::reset_for_tests();

    assert!(!settings::is_initialized());

    settings::initialize(Config::default()).unwrap();
    settings::reset_for_tests();

    assert!(!settings::is_initialized());
}
