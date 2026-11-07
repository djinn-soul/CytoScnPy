//! Open documents, what to analyze, and publishing the results.
//!
//! Like the VS Code extension, a file's diagnostics are cleared while its
//! buffer differs from the text they were found in, since their positions
//! would be wrong, and come back when an undo restores that text.

use std::path::{Path, PathBuf};

use lsp_types::{DidChangeWatchedFilesParams, MessageType, Uri};

use crate::client::BoxError;
use crate::diagnostics::Entry;
use crate::server::Server;

pub(crate) struct Document {
    path: Option<PathBuf>,
    /// `path` with symlinks resolved, which project results are keyed by.
    canonical: Option<PathBuf>,
    pub text: String,
    /// The text the last analysis read and the diagnostics it produced.
    pub analysis: Option<(String, Vec<Entry>)>,
    /// Whether the diagnostics were cleared because the buffer was edited.
    cleared: bool,
}

impl Document {
    pub(crate) fn new(uri: &Uri, text: String) -> Self {
        let path = crate::uri::to_path(uri.as_str());
        Self {
            canonical: path
                .as_ref()
                .map(|path| path.canonicalize().unwrap_or_else(|_| path.clone())),
            path,
            text,
            analysis: None,
            cleared: false,
        }
    }

    /// The diagnostics of the last analysis, while the buffer still holds
    /// the text it read; their fixes would be stale otherwise.
    pub(crate) fn fresh_entries(&self) -> Option<&[Entry]> {
        self.analysis
            .as_ref()
            .filter(|(text, _)| *text == self.text)
            .map(|(_, entries)| entries.as_slice())
    }

    pub(crate) fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    pub(crate) fn is_dirty(&self) -> bool {
        self.path
            .as_ref()
            .and_then(|path| std::fs::read_to_string(path).ok())
            .is_some_and(|saved| saved != self.text)
    }
}

/// What one analysis covers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Target {
    /// One project in a workspace folder.
    Project(PathBuf),
    /// One open document, analyzed alone.
    File(Uri),
    /// Every project and open document, from scratch.
    Workspace,
}

impl Server<'_> {
    /// The project holding the document, or the document alone in file
    /// mode or outside every workspace folder.
    pub(crate) fn target_of(&mut self, uri: &Uri) -> Option<Target> {
        let document = self.documents.get(uri)?;
        document.path.as_ref()?; // Unsaved or remote buffers have no file.
        let file = document.canonical.clone()?;
        if !self.settings().file_mode {
            if let Some(root) = self.projects.project_of(&file) {
                return Some(Target::Project(root));
            }
        }
        Some(Target::File(uri.clone()))
    }

    pub(crate) fn enqueue(&mut self, target: impl Into<Option<Target>>) {
        if let Some(target) = target.into() {
            if !self.queue.contains(&target) {
                self.queue.push(target);
            }
        }
    }

    pub(crate) fn on_open(&mut self, uri: &Uri) -> Result<(), BoxError> {
        match self.target_of(uri) {
            // A project analyzed before already holds the file's findings.
            Some(Target::Project(root)) if self.projects.results(&root).is_some() => {
                self.sync_documents(&root);
                self.show_document(uri)
            }
            target => {
                self.enqueue(target);
                Ok(())
            }
        }
    }

    pub(crate) fn on_edit(&mut self, uri: &Uri, text: String) -> Result<(), BoxError> {
        if let Some(document) = self.documents.get_mut(uri) {
            document.text = text;
            let current = document.fresh_entries().is_some();
            // Publish only when the state flips, not on every keystroke.
            if document.analysis.is_some() && current == document.cleared {
                return self.show_document(uri);
            }
        }
        Ok(())
    }

    pub(crate) fn on_close(&mut self, uri: Uri) -> Result<(), BoxError> {
        let Some(document) = self.documents.remove(&uri) else {
            return Ok(());
        };
        let project = (!self.settings().file_mode)
            .then_some(document.canonical.as_deref())
            .flatten()
            .and_then(|file| self.projects.project_of(file).map(|root| (root, file)));
        match project {
            // Project findings stay listed for closed files, as in VS Code,
            // as found on disk rather than in the discarded buffer.
            Some((root, file)) => {
                let found = self.file_entries(&root, file);
                self.client.publish(uri, diagnostics(&found))
            }
            None => self.client.publish(uri, Vec::new()),
        }
    }

    /// Re-analyzes projects whose files changed on disk, and everything
    /// when a configuration file changed.
    pub(crate) fn on_files_changed(&mut self, params: &DidChangeWatchedFilesParams) {
        for change in &params.changes {
            let Some(path) = crate::uri::to_path(change.uri.as_str()) else {
                continue;
            };
            let name = path.file_name().and_then(|name| name.to_str());
            if matches!(name, Some("pyproject.toml" | ".cytoscnpy.toml")) {
                self.enqueue(Target::Workspace);
                continue;
            }
            let file = path.canonicalize().unwrap_or(path);
            // A deleted file no longer resolves; its folder still matches.
            if !self.settings().file_mode {
                if let Some(root) = self.projects.project_of(&file) {
                    self.enqueue(Target::Project(root));
                }
            }
        }
    }

    /// Gives open documents in `root` the findings the last analysis made
    /// in them, for their code actions and diagnostics.
    pub(crate) fn sync_documents(&mut self, root: &Path) {
        let results = self.projects.results(root);
        for document in self.documents.values_mut() {
            let Some(file) = &document.canonical else {
                continue;
            };
            if !file.starts_with(root) {
                continue;
            }
            document.analysis = Some(match results.and_then(|files| files.get(file)) {
                Some(result) => (result.text.clone(), result.entries.clone()),
                // No findings: on disk, the file is clean.
                None => (
                    std::fs::read_to_string(file).unwrap_or_else(|_| document.text.clone()),
                    Vec::new(),
                ),
            });
        }
    }

    pub(crate) fn file_entries(&self, root: &Path, file: &Path) -> Vec<Entry> {
        self.projects
            .results(root)
            .and_then(|files| files.get(file))
            .map(|result| result.entries.clone())
            .unwrap_or_default()
    }

    /// Publishes a project file's findings, through its open document when
    /// there is one.
    pub(crate) fn publish_file(&mut self, file: &Path, found: &[Entry]) -> Result<(), BoxError> {
        let open = self
            .documents
            .iter()
            .find(|(_, document)| document.canonical.as_deref() == Some(file))
            .map(|(uri, _)| uri.clone());
        match open {
            Some(uri) => self.show_document(&uri),
            None => match crate::uri::from_path(file) {
                Some(uri) => self.client.publish(uri, diagnostics(found)),
                None => Ok(()),
            },
        }
    }

    /// Publishes a document's findings, or none while its buffer differs
    /// from the text they were found in.
    pub(crate) fn show_document(&mut self, uri: &Uri) -> Result<(), BoxError> {
        let Some(document) = self.documents.get_mut(uri) else {
            return Ok(());
        };
        let shown = document.fresh_entries().map(diagnostics);
        document.cleared = shown.is_none();
        self.client.publish(uri.clone(), shown.unwrap_or_default())
    }

    pub(crate) fn warn(&self, message: &str) -> Result<(), BoxError> {
        self.client.show(MessageType::WARNING, message.to_owned())
    }
}

pub(crate) fn diagnostics(found: &[Entry]) -> Vec<lsp_types::Diagnostic> {
    found.iter().map(|entry| entry.diagnostic.clone()).collect()
}
