use std::collections::HashSet;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Return repository-relative paths committed at HEAD. An unavailable inventory
/// must not make uncommitted files look like mature, frozen modules.
pub(crate) fn tracked_paths(git_root: &Path) -> io::Result<HashSet<PathBuf>> {
    let output = Command::new("git")
        .args(["ls-tree", "-r", "-z", "--name-only", "HEAD"])
        .current_dir(git_root)
        .output()?;
    if !output.status.success() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("git ls-tree failed with {}", output.status),
        ));
    }
    let paths = String::from_utf8(output.stdout)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
    Ok(paths
        .split('\0')
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
        .collect())
}
