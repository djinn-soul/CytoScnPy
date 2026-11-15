//! The VS Code extension's commands, as LSP commands.
//!
//! Editors without a command palette for language servers reach them as
//! source actions: on a line without a CytoScnPy finding, or when the
//! client asks for source actions.

use lsp_types::{
    CodeAction, CodeActionContext, CodeActionKind, CodeActionOrCommand, Command,
    ExecuteCommandParams, MessageType, Uri,
};

use crate::client::BoxError;
use crate::diagnostics::SOURCE;
use crate::server::Server;
use crate::workspace::Target;

/// Command IDs and titles, matching the VS Code extension's.
pub(crate) const COMMANDS: [(&str, &str); 6] = [
    (
        "cytoscnpy.analyzeCurrentFile",
        "CytoScnPy: Analyze Current File",
    ),
    ("cytoscnpy.analyzeWorkspace", "CytoScnPy: Analyze Workspace"),
    (
        "cytoscnpy.complexity",
        "CytoScnPy: Calculate Cyclomatic Complexity (cc)",
    ),
    (
        "cytoscnpy.halstead",
        "CytoScnPy: Calculate Halstead Metrics (hal)",
    ),
    (
        "cytoscnpy.maintainability",
        "CytoScnPy: Calculate Maintainability Index (mi)",
    ),
    (
        "cytoscnpy.rawMetrics",
        "CytoScnPy: Calculate Raw Metrics (raw)",
    ),
];

/// The commands as source actions on `uri`, unless the request is about a
/// CytoScnPy finding and did not ask for source actions; there, the fixes
/// come first and these would push them off the menu.
pub(crate) fn source_actions(uri: &Uri, context: &CodeActionContext) -> Vec<CodeActionOrCommand> {
    let asked = context.only.as_ref().is_some_and(|only| {
        only.iter()
            .any(|kind| kind.as_str().starts_with(CodeActionKind::SOURCE.as_str()))
    });
    let on_finding = context
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.source.as_deref() == Some(SOURCE));
    if on_finding && !asked {
        return Vec::new();
    }
    COMMANDS
        .iter()
        .map(|(id, title)| {
            CodeActionOrCommand::CodeAction(CodeAction {
                title: (*title).to_owned(),
                kind: Some(CodeActionKind::SOURCE),
                command: Some(Command {
                    title: (*title).to_owned(),
                    command: (*id).to_owned(),
                    arguments: Some(vec![serde_json::json!(uri.as_str())]),
                }),
                ..CodeAction::default()
            })
        })
        .collect()
}

impl Server<'_> {
    pub(crate) fn execute(&mut self, params: &ExecuteCommandParams) -> Result<(), BoxError> {
        let uri = params
            .arguments
            .first()
            .and_then(serde_json::Value::as_str)
            .and_then(|uri| uri.parse::<Uri>().ok());
        let metric = match params.command.as_str() {
            "cytoscnpy.analyzeWorkspace" => {
                self.enqueue(Target::Workspace);
                return self.run_queue();
            }
            "cytoscnpy.analyzeCurrentFile" => return self.analyze_current(uri.as_ref()),
            "cytoscnpy.complexity" => ("cc", "Cyclomatic Complexity"),
            "cytoscnpy.halstead" => ("hal", "Halstead Metrics"),
            "cytoscnpy.maintainability" => ("mi", "Maintainability Index"),
            "cytoscnpy.rawMetrics" => ("raw", "Raw Metrics"),
            other => return self.warn(&format!("Unknown CytoScnPy command {other}")),
        };
        self.run_metric(uri.as_ref(), metric)
    }

    /// Like VS Code: a buffer with unsaved changes is not analyzed, and a
    /// file in a project is analyzed with the rest of it.
    fn analyze_current(&mut self, uri: Option<&Uri>) -> Result<(), BoxError> {
        let Some(uri) = uri.filter(|uri| self.documents.contains_key(*uri)) else {
            return self.warn("No active text editor to analyze.");
        };
        if self.documents[uri].is_dirty() {
            return self.warn("Save the Python file before running CytoScnPy analysis.");
        }
        let target = self.target_of(uri);
        self.enqueue(target);
        self.run_queue()
    }

    fn run_metric(
        &mut self,
        uri: Option<&Uri>,
        (metric, name): (&str, &str),
    ) -> Result<(), BoxError> {
        let path = uri
            .and_then(|uri| self.documents.get(uri))
            .and_then(|document| document.path().map(std::path::Path::to_path_buf));
        let Some(path) = path else {
            return self.warn(&format!("No active Python file to run {name} on."));
        };
        match crate::analysis::metric_report(metric, &path) {
            Ok(report) => {
                let stem = path.file_stem().unwrap_or_default().to_string_lossy();
                let text = format!("CytoScnPy {name}: {}\n\n{report}", path.display());
                self.client.show_text(&format!("{stem}-{metric}.txt"), text)
            }
            Err(message) => {
                self.client.log(MessageType::ERROR, message.clone())?;
                self.client.show(
                    MessageType::ERROR,
                    format!("CytoScnPy {name} failed: {message}"),
                )
            }
        }
    }
}
