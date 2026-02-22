mod loader;
mod models;
mod security;
mod whitelist;

use std::path::Path;

pub use models::{Config, CytoScnPyConfig, ProjectType};
pub use security::{CustomSecretPattern, DangerConfig, SecretsConfig};
pub use whitelist::{get_builtin_whitelists, WhitelistEntry, WhitelistPattern};

impl Config {
    /// Loads configuration from default locations (.cytoscnpy.toml or pyproject.toml in current dir).
    #[must_use]
    pub fn load() -> Self {
        Self::load_from_path(Path::new("."))
    }

    /// Loads configuration starting from a specific path and traversing up.
    #[must_use]
    pub fn load_from_path(path: &Path) -> Self {
        loader::load_from_path(path)
    }
}

#[cfg(test)]
mod tests;
