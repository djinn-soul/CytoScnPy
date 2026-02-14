/// A RAII guard that restores the current working directory when dropped.
///
/// This is useful for tests that need to change the CWD but want to ensure
/// it's restored even if the test panics.
pub struct CwdGuard {
    original_cwd: std::path::PathBuf,
}

impl CwdGuard {
    /// Creates a new `CwdGuard` and changes the CWD to the specified path.
    ///
    /// # Errors
    ///
    /// Returns an error if getting the current directory or setting the new one fails.
    pub fn new<P: AsRef<std::path::Path>>(new_cwd: P) -> anyhow::Result<Self> {
        let original_cwd = std::env::current_dir()?;
        std::env::set_current_dir(new_cwd)?;
        Ok(Self { original_cwd })
    }
}

impl Drop for CwdGuard {
    fn drop(&mut self) {
        if let Err(e) = std::env::set_current_dir(&self.original_cwd) {
            eprintln!(
                "Failed to restore CWD to {}: {}",
                self.original_cwd.display(),
                e
            );
        }
    }
}
