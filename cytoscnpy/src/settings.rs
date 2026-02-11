use crate::config::Config;
use std::fmt;
use std::sync::{Arc, Mutex, MutexGuard};

static SETTINGS: Mutex<Option<Arc<Config>>> = Mutex::new(None);

/// Errors emitted when interacting with the global settings store.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingsError {
    /// The settings store was already initialized with a `Config`.
    AlreadyInitialized,
    /// The settings store has not been initialized yet.
    NotInitialized,
}

impl fmt::Display for SettingsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SettingsError::AlreadyInitialized => {
                write!(f, "global settings have already been initialized")
            }
            SettingsError::NotInitialized => {
                write!(
                    f,
                    "settings::initialize must be called before reading the global config"
                )
            }
        }
    }
}

impl std::error::Error for SettingsError {}

/// Initializes the global settings store.
///
/// Returns an error if initialization has already happened.
pub fn initialize(config: Config) -> Result<(), SettingsError> {
    let mut guard = lock_settings();
    if guard.is_some() {
        return Err(SettingsError::AlreadyInitialized);
    }
    *guard = Some(Arc::new(config));
    Ok(())
}

/// Returns a clone of the currently stored `Config`.
///
pub fn config() -> Result<Arc<Config>, SettingsError> {
    try_config().ok_or(SettingsError::NotInitialized)
}

/// Returns the stored `Config` if the store has been initialized.
#[must_use]
pub fn try_config() -> Option<Arc<Config>> {
    lock_settings().clone()
}

/// Returns `true` once the store has been initialized.
#[must_use]
pub fn is_initialized() -> bool {
    lock_settings().is_some()
}

/// Resets the singleton between tests.
pub fn reset_for_tests() {
    *lock_settings() = None;
}

fn lock_settings() -> MutexGuard<'static, Option<Arc<Config>>> {
    match SETTINGS.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}
