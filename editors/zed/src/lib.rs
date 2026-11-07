//! Zed extension that runs `cytoscnpy lsp` for Python files.
//!
//! The server binary comes from the `lsp.cytoscnpy.binary.path` setting when
//! one is set, and otherwise from the latest CytoScnPy GitHub release, which
//! the extension downloads once per version.

use std::fs;

use zed_extension_api::{
    self as zed, settings::LspSettings, Architecture, DownloadedFileType, GithubReleaseOptions,
    LanguageServerId, LanguageServerInstallationStatus, Os, Result, Worktree,
};

const SERVER_NAME: &str = "cytoscnpy";
const REPOSITORY: &str = "djinn-soul/CytoScnPy";
/// Downloaded releases live in `cytoscnpy-<version>/` in the extension's
/// work directory.
const VERSION_DIR_PREFIX: &str = "cytoscnpy-";

struct CytoScnPyExtension {
    cached_binary: Option<String>,
}

impl CytoScnPyExtension {
    /// The release asset built for this platform.
    fn asset_name() -> Result<&'static str> {
        match zed::current_platform() {
            (Os::Linux, Architecture::X8664) => Ok("cytoscnpy-linux-x64"),
            (Os::Mac, Architecture::Aarch64) => Ok("cytoscnpy-macos-arm64"),
            (Os::Mac, Architecture::X8664) => Ok("cytoscnpy-macos-x64"),
            (Os::Windows, Architecture::X8664) => Ok("cytoscnpy-windows-x64.exe"),
            (os, arch) => Err(format!(
                "CytoScnPy publishes no binary for {os:?} on {arch:?}. Build it with \
                 `cargo install --git https://github.com/{REPOSITORY} cytoscnpy-cli` and \
                 set `lsp.cytoscnpy.binary.path` to the installed `cytoscnpy-cli`."
            )),
        }
    }

    /// The path of the downloaded release binary, downloading the latest
    /// release when it is not on disk yet.
    fn release_binary(&mut self, id: &LanguageServerId) -> Result<String> {
        if let Some(path) = &self.cached_binary {
            if is_file(path) {
                return Ok(path.clone());
            }
        }
        let asset_name = Self::asset_name()?;
        zed::set_language_server_installation_status(
            id,
            &LanguageServerInstallationStatus::CheckingForUpdate,
        );
        let options = GithubReleaseOptions {
            require_assets: true,
            pre_release: false,
        };
        let release = match zed::latest_github_release(REPOSITORY, options) {
            Ok(release) => release,
            // Offline: keep using a release downloaded earlier.
            Err(error) => return downloaded_binary(asset_name).ok_or(error),
        };
        let version_dir = format!("{VERSION_DIR_PREFIX}{}", release.version);
        let binary = format!("{version_dir}/{asset_name}");
        if !is_file(&binary) {
            let asset = release
                .assets
                .iter()
                .find(|asset| asset.name == asset_name)
                .ok_or_else(|| {
                    format!("CytoScnPy release {} has no {asset_name}", release.version)
                })?;
            zed::set_language_server_installation_status(
                id,
                &LanguageServerInstallationStatus::Downloading,
            );
            fs::create_dir_all(&version_dir)
                .map_err(|error| format!("Cannot create {version_dir}: {error}"))?;
            zed::download_file(
                &asset.download_url,
                &binary,
                DownloadedFileType::Uncompressed,
            )
            .map_err(|error| format!("Cannot download {asset_name}: {error}"))?;
            zed::make_file_executable(&binary)?;
            remove_other_versions(&version_dir);
        }
        zed::set_language_server_installation_status(id, &LanguageServerInstallationStatus::None);
        self.cached_binary = Some(binary.clone());
        Ok(binary)
    }
}

impl zed::Extension for CytoScnPyExtension {
    fn new() -> Self {
        Self {
            cached_binary: None,
        }
    }

    fn language_server_command(
        &mut self,
        id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<zed::Command> {
        let binary = LspSettings::for_worktree(SERVER_NAME, worktree)
            .ok()
            .and_then(|settings| settings.binary);
        let configured_path = binary.as_ref().and_then(|binary| binary.path.clone());
        let args = binary
            .as_ref()
            .and_then(|binary| binary.arguments.clone())
            .unwrap_or_else(|| vec!["lsp".to_owned()]);
        let mut env = binary.and_then(|binary| binary.env).unwrap_or_default();
        let command = match configured_path {
            Some(path) => {
                // A configured binary may rely on the shell's PATH.
                let mut shell_env: Vec<_> = worktree.shell_env();
                shell_env.extend(env.drain());
                return Ok(zed::Command {
                    command: path,
                    args,
                    env: shell_env,
                });
            }
            None => self.release_binary(id)?,
        };
        Ok(zed::Command {
            command,
            args,
            env: env.into_iter().collect(),
        })
    }

    fn language_server_initialization_options(
        &mut self,
        _id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<Option<zed::serde_json::Value>> {
        Ok(LspSettings::for_worktree(SERVER_NAME, worktree)
            .ok()
            .and_then(|settings| settings.initialization_options))
    }
}

fn is_file(path: &str) -> bool {
    fs::metadata(path).is_ok_and(|metadata| metadata.is_file())
}

/// A binary from an earlier download, newest version directory first.
fn downloaded_binary(asset_name: &str) -> Option<String> {
    let mut dirs: Vec<String> = fs::read_dir(".")
        .ok()?
        .filter_map(|entry| entry.ok()?.file_name().into_string().ok())
        .filter(|name| name.starts_with(VERSION_DIR_PREFIX))
        .collect();
    dirs.sort_unstable();
    dirs.into_iter()
        .rev()
        .map(|dir| format!("{dir}/{asset_name}"))
        .find(|binary| is_file(binary))
}

fn remove_other_versions(keep: &str) {
    let Ok(entries) = fs::read_dir(".") else {
        return;
    };
    for entry in entries.flatten() {
        let Ok(name) = entry.file_name().into_string() else {
            continue;
        };
        if name.starts_with(VERSION_DIR_PREFIX) && name != keep {
            fs::remove_dir_all(entry.path()).ok();
        }
    }
}

zed::register_extension!(CytoScnPyExtension);
