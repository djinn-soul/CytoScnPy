//! Running analyses: of every project, one project, or one file.

use std::path::Path;

use lsp_types::Uri;

use crate::client::BoxError;
use crate::diagnostics::entries;
use crate::server::Server;
use crate::workspace::Target;

impl Server<'_> {
    /// Runs the analyses the debounce held back.
    pub(crate) fn run_queue(&mut self) -> Result<(), BoxError> {
        let mut queue = std::mem::take(&mut self.queue);
        if queue.contains(&Target::Workspace) {
            queue = vec![Target::Workspace];
        }
        for target in queue {
            match target {
                Target::Workspace => self.analyze_workspace()?,
                Target::Project(root) => self.analyze_project(&root)?,
                Target::File(uri) => self.analyze_file(&uri)?,
            }
        }
        Ok(())
    }

    /// Analyzes every project and every open document from scratch, like
    /// the VS Code extension's "Analyze Workspace".
    pub(crate) fn analyze_workspace(&mut self) -> Result<(), BoxError> {
        let previous = self.projects.reset();
        let roots = if self.settings().file_mode {
            Vec::new()
        } else {
            self.projects.all_projects()
        };
        for root in &roots {
            self.analyze_project(root)?;
        }
        for file in previous {
            let covered = roots.iter().any(|root| {
                self.projects
                    .results(root)
                    .is_some_and(|files| files.contains_key(&file))
            });
            if !covered {
                self.publish_file(&file, &[])?;
            }
        }
        let uris: Vec<Uri> = self.documents.keys().cloned().collect();
        for uri in uris {
            if let Some(Target::File(uri)) = self.target_of(&uri) {
                self.analyze_file(&uri)?;
            }
        }
        Ok(())
    }

    fn analyze_project(&mut self, root: &Path) -> Result<(), BoxError> {
        let name = root.file_name().map_or_else(
            || root.display().to_string(),
            |name| name.to_string_lossy().into_owned(),
        );
        let progress = self.client.progress_begin(format!("Analyzing {name}"))?;
        let args = self.settings().args;
        let changed = match self.projects.analyze(root, &args) {
            Ok(changed) => changed,
            Err(message) => {
                self.client.progress_end(progress, "Failed".to_owned())?;
                return self.client.report_failure(&message);
            }
        };
        self.client.report_success();
        let total: usize = self.projects.results(root).map_or(0, |files| {
            files.values().map(|file| file.entries.len()).sum()
        });
        self.client
            .progress_end(progress, format!("{total} findings"))?;
        self.sync_documents(root);
        for file in changed {
            let found = self.file_entries(root, &file);
            self.publish_file(&file, &found)?;
        }
        Ok(())
    }

    fn analyze_file(&mut self, uri: &Uri) -> Result<(), BoxError> {
        let Some(path) = self
            .documents
            .get(uri)
            .and_then(|d| d.path().map(Path::to_path_buf))
        else {
            return Ok(());
        };
        let progress = self.client.progress_begin(format!(
            "Analyzing {}",
            path.file_name().unwrap_or_default().to_string_lossy()
        ))?;
        match crate::analysis::analyze(&path, &self.settings().args) {
            Ok(analysis) => {
                self.client.report_success();
                let found = entries(&analysis.report, &analysis.text);
                self.client
                    .progress_end(progress, format!("{} findings", found.len()))?;
                if let Some(document) = self.documents.get_mut(uri) {
                    document.analysis = Some((analysis.text, found));
                }
                self.show_document(uri)
            }
            Err(message) => {
                self.client.progress_end(progress, "Failed".to_owned())?;
                self.client.report_failure(&message)
            }
        }
    }
}
