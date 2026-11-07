//! Whole-project analysis, like the VS Code extension's default workspace
//! mode.
//!
//! Analyzing one file alone reports a function as unused when only another
//! file calls it. A file inside a workspace folder is analyzed together with
//! the rest of that folder instead, and the findings are split by file.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use lsp_types::InitializeParams;

use crate::diagnostics::{entries, Entry};

/// The findings in one file, with the text their positions refer to.
pub(crate) struct FileResult {
    pub text: String,
    pub entries: Vec<Entry>,
}

/// Workspace folders, the projects in them, and the last analysis of each
/// project.
#[derive(Default)]
pub(crate) struct Projects {
    folders: Vec<PathBuf>,
    /// Per workspace folder, its project roots, found when first needed.
    roots: HashMap<PathBuf, Vec<PathBuf>>,
    /// Per analyzed project root, the files that have findings.
    results: HashMap<PathBuf, HashMap<PathBuf, FileResult>>,
}

impl Projects {
    /// The workspace folders the client opened.
    #[allow(deprecated)] // `root_uri` is the fallback for older clients.
    pub(crate) fn new(params: &InitializeParams) -> Self {
        let uris: Vec<&lsp_types::Uri> = match &params.workspace_folders {
            Some(folders) if !folders.is_empty() => {
                folders.iter().map(|folder| &folder.uri).collect()
            }
            _ => params.root_uri.iter().collect(),
        };
        let folders = uris
            .into_iter()
            .filter_map(|uri| crate::uri::to_path(uri.as_str()))
            .filter_map(|path| path.canonicalize().ok())
            .filter(|path| path.is_dir())
            .collect();
        Self {
            folders,
            ..Self::default()
        }
    }

    /// The innermost workspace folder holding `file`, a canonical path.
    pub(crate) fn folder_of(&self, file: &Path) -> Option<&Path> {
        innermost(&self.folders, file)
    }

    /// The innermost project holding `file`, a canonical path, in its
    /// workspace folder.
    pub(crate) fn project_of(&mut self, file: &Path) -> Option<PathBuf> {
        let folder = self.folder_of(file)?.to_path_buf();
        let roots = self
            .roots
            .entry(folder.clone())
            .or_insert_with(|| crate::analysis::project_roots(&folder));
        Some(innermost(roots, file).map_or(folder, Path::to_path_buf))
    }

    /// Every project in every workspace folder.
    pub(crate) fn all_projects(&mut self) -> Vec<PathBuf> {
        let folders = self.folders.clone();
        let mut all = Vec::new();
        for folder in folders {
            let roots = self
                .roots
                .entry(folder.clone())
                .or_insert_with(|| crate::analysis::project_roots(&folder));
            all.extend(roots.iter().cloned());
        }
        all
    }

    /// Forgets the projects found and the results, for example after the
    /// settings change, and returns the files that had findings.
    pub(crate) fn reset(&mut self) -> Vec<PathBuf> {
        self.roots.clear();
        self.results
            .drain()
            .flat_map(|(_, files)| files.into_keys())
            .collect()
    }

    /// The last analysis of `root`, if it has been analyzed.
    pub(crate) fn results(&self, root: &Path) -> Option<&HashMap<PathBuf, FileResult>> {
        self.results.get(root)
    }

    /// Analyzes `root` and returns the files whose diagnostics may have
    /// changed: those with findings now, and those that had findings before.
    ///
    /// # Errors
    ///
    /// Returns a message when the analysis fails; the last results stay.
    pub(crate) fn analyze(
        &mut self,
        root: &Path,
        extra_args: &[String],
    ) -> Result<Vec<PathBuf>, String> {
        let report = crate::analysis::analyze_project(root, extra_args)?;
        let mut files = HashMap::new();
        for (file, report) in report.by_file() {
            // Notebook findings sit in cell coordinates that a `.ipynb`
            // buffer does not share; editors open Python files here.
            if !matches!(
                file.extension().and_then(|ext| ext.to_str()),
                Some("py" | "pyi")
            ) {
                continue;
            }
            // The file is read after the analysis. If it changed in between,
            // the open buffer will not match this text, and its fixes are
            // withheld until the next save.
            let Ok(text) = std::fs::read_to_string(&file) else {
                continue;
            };
            let found = entries(&report, &text);
            let file = file.canonicalize().unwrap_or(file);
            files.insert(
                file,
                FileResult {
                    text,
                    entries: found,
                },
            );
        }
        let previous = self.results.insert(root.to_path_buf(), files);
        let current = &self.results[root];
        let mut changed: Vec<PathBuf> = current.keys().cloned().collect();
        changed.extend(
            previous
                .into_iter()
                .flat_map(HashMap::into_keys)
                .filter(|file| !current.contains_key(file)),
        );
        Ok(changed)
    }
}

/// The longest of `roots` that holds `file`.
fn innermost<'a>(roots: &'a [PathBuf], file: &Path) -> Option<&'a Path> {
    roots
        .iter()
        .filter(|root| file.starts_with(root))
        .max_by_key(|root| root.as_os_str().len())
        .map(PathBuf::as_path)
}
