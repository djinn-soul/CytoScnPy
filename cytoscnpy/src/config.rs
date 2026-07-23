mod loader;
mod models;
mod security;
mod whitelist;

use std::path::Path;

pub use models::{Config, CytoScnPyConfig, ProjectType};
pub use security::{
    CustomSecretPattern, DangerConfig, SanitizerConfig, SanitizerGroup, SecretsConfig,
};
pub use whitelist::{get_builtin_whitelists, WhitelistEntry, WhitelistPattern};

impl Config {
    /// Loads configuration from the current directory, falling back to defaults on errors.
    ///
    /// Use [`Self::try_load`] when configuration errors must be reported.
    #[must_use]
    pub fn load() -> Self {
        Self::load_from_path(Path::new("."))
    }

    /// Loads configuration from a path, falling back to defaults on read or parse errors.
    ///
    /// This preserves the legacy infallible API. New callers should prefer
    /// [`Self::try_load_from_path`] when configuration errors must be reported.
    #[must_use]
    pub fn load_from_path(path: &Path) -> Self {
        loader::load_from_path(path)
    }

    /// Loads configuration from the current directory and reports errors.
    ///
    /// # Errors
    ///
    /// Returns an error when a discovered configuration file cannot be read or parsed.
    pub fn try_load() -> anyhow::Result<Self> {
        Self::try_load_from_path(Path::new("."))
    }

    /// Loads configuration while preserving file read and TOML parsing errors.
    ///
    /// # Errors
    ///
    /// Returns an error when a discovered configuration file cannot be read or parsed.
    pub fn try_load_from_path(path: &Path) -> anyhow::Result<Self> {
        loader::try_load_from_path(path)
    }
}

#[cfg(test)]
mod tests;
