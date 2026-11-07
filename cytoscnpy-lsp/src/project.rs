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

/// Workspace folders and the last analysis of each.
#[derive(Default)]
pub(crate) struct Projects {
    roots: Vec<PathBuf>,
    /// Per analyzed root, the files that have findings.
    results: HashMap<PathBuf, HashMap<PathBuf, FileResult>>,
}

impl Projects {
    /// The workspace folders the client opened, or none when the client
    /// asks for single-file analysis with `{"analysisMode": "file"}`.
    #[allow(deprecated)] // `root_uri` is the fallback for older clients.
    pub(crate) fn new(params: &InitializeParams) -> Self {
        let file_mode = params
            .initialization_options
            .as_ref()
            .and_then(|options| options.get("analysisMode"))
            .and_then(serde_json::Value::as_str)
            == Some("file");
        if file_mode {
            return Self::default();
        }
        let uris: Vec<&lsp_types::Uri> = match &params.workspace_folders {
            Some(folders) if !folders.is_empty() => {
                folders.iter().map(|folder| &folder.uri).collect()
            }
            _ => params.root_uri.iter().collect(),
        };
        let roots = uris
            .into_iter()
            .filter_map(|uri| crate::uri::to_path(uri.as_str()))
            .filter_map(|path| path.canonicalize().ok())
            .filter(|path| path.is_dir())
            .collect();
        Self {
            roots,
            results: HashMap::new(),
        }
    }

    /// The innermost workspace folder holding `file`, a canonical path.
    pub(crate) fn root_of(&self, file: &Path) -> Option<&Path> {
        self.roots
            .iter()
            .filter(|root| file.starts_with(root))
            .max_by_key(|root| root.as_os_str().len())
            .map(PathBuf::as_path)
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
