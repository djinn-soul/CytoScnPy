//! The LSP message loop and document state.

use std::collections::HashMap;
use std::error::Error;
use std::path::{Path, PathBuf};

use lsp_server::{Connection, ErrorCode, Message, Notification, Request, Response};
use lsp_types::notification::{
    DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, DidSaveTextDocument,
    LogMessage, Notification as _, PublishDiagnostics,
};
use lsp_types::request::{CodeActionRequest, Request as _};
use lsp_types::{
    CodeActionKind, CodeActionOptions, CodeActionParams, CodeActionProviderCapability,
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    DidSaveTextDocumentParams, InitializeParams, InitializeResult, LogMessageParams, MessageType,
    PublishDiagnosticsParams, SaveOptions, ServerCapabilities, ServerInfo,
    TextDocumentSyncCapability, TextDocumentSyncKind, TextDocumentSyncOptions,
    TextDocumentSyncSaveOptions, Uri,
};

use crate::actions::{code_actions, Context};
use crate::diagnostics::{entries, Entry};
use crate::project::Projects;

type BoxError = Box<dyn Error + Send + Sync>;

struct Document {
    path: Option<PathBuf>,
    /// `path` with symlinks resolved, which project results are keyed by.
    canonical: Option<PathBuf>,
    text: String,
    /// The text the last analysis read and the diagnostics it produced.
    analysis: Option<(String, Vec<Entry>)>,
}

struct Server<'a> {
    connection: &'a Connection,
    documents: HashMap<Uri, Document>,
    /// Extra `cytoscnpy` CLI flags from the `args` initialization option.
    extra_args: Vec<String>,
    projects: Projects,
}

/// Serves LSP on `connection` until the client shuts the server down.
///
/// The client may pass `{"args": ["--secrets", ...]}` as initialization
/// options to add CLI flags to every analysis.
///
/// # Errors
///
/// Returns an error when the connection fails or the client breaks the
/// protocol.
pub fn run(connection: &Connection) -> Result<(), BoxError> {
    let (id, params) = connection.initialize_start()?;
    let params: InitializeParams = serde_json::from_value(params)?;
    let result = InitializeResult {
        capabilities: capabilities(),
        server_info: Some(ServerInfo {
            name: "cytoscnpy".to_owned(),
            version: Some(env!("CARGO_PKG_VERSION").to_owned()),
        }),
    };
    connection.initialize_finish(id, serde_json::to_value(result)?)?;
    let mut server = Server {
        connection,
        documents: HashMap::new(),
        extra_args: extra_args(params.initialization_options.as_ref()),
        projects: Projects::new(&params),
    };
    for message in &connection.receiver {
        match message {
            Message::Request(request) => {
                if connection.handle_shutdown(&request)? {
                    return Ok(());
                }
                server.on_request(request)?;
            }
            Message::Notification(notification) => server.on_notification(notification)?,
            Message::Response(_) => {}
        }
    }
    Ok(())
}

fn capabilities() -> ServerCapabilities {
    ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Options(
            TextDocumentSyncOptions {
                open_close: Some(true),
                change: Some(TextDocumentSyncKind::FULL),
                save: Some(TextDocumentSyncSaveOptions::SaveOptions(SaveOptions {
                    include_text: Some(false),
                })),
                ..TextDocumentSyncOptions::default()
            },
        )),
        code_action_provider: Some(CodeActionProviderCapability::Options(CodeActionOptions {
            code_action_kinds: Some(vec![CodeActionKind::QUICKFIX]),
            ..CodeActionOptions::default()
        })),
        ..ServerCapabilities::default()
    }
}

fn extra_args(options: Option<&serde_json::Value>) -> Vec<String> {
    options
        .and_then(|options| options.get("args"))
        .and_then(serde_json::Value::as_array)
        .map(|args| {
            args.iter()
                .filter_map(|arg| arg.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

impl Server<'_> {
    fn on_request(&self, request: Request) -> Result<(), BoxError> {
        let response = if request.method == CodeActionRequest::METHOD {
            let (id, params) = request.extract::<CodeActionParams>(CodeActionRequest::METHOD)?;
            let actions = self
                .documents
                .get(&params.text_document.uri)
                .map(|document| {
                    let context = Context {
                        uri: &params.text_document.uri,
                        text: &document.text,
                        fresh_entries: document
                            .analysis
                            .as_ref()
                            .filter(|(text, _)| *text == document.text)
                            .map(|(_, entries)| entries.as_slice()),
                    };
                    code_actions(&context, params.range, &params.context.diagnostics)
                });
            Response::new_ok(id, actions.unwrap_or_default())
        } else {
            let message = format!("cytoscnpy does not handle {}", request.method);
            Response::new_err(request.id, ErrorCode::MethodNotFound as i32, message)
        };
        self.connection.sender.send(response.into())?;
        Ok(())
    }

    fn on_notification(&mut self, notification: Notification) -> Result<(), BoxError> {
        match notification.method.as_str() {
            DidOpenTextDocument::METHOD => {
                let params: DidOpenTextDocumentParams =
                    notification.extract(DidOpenTextDocument::METHOD)?;
                let uri = params.text_document.uri;
                let path = crate::uri::to_path(uri.as_str());
                let document = Document {
                    canonical: path
                        .as_ref()
                        .map(|path| path.canonicalize().unwrap_or_else(|_| path.clone())),
                    path,
                    text: params.text_document.text,
                    analysis: None,
                };
                self.documents.insert(uri.clone(), document);
                self.analyze(&uri, true)?;
            }
            DidChangeTextDocument::METHOD => {
                let params: DidChangeTextDocumentParams =
                    notification.extract(DidChangeTextDocument::METHOD)?;
                // Full sync: the last change holds the whole text.
                let text = params
                    .content_changes
                    .into_iter()
                    .last()
                    .map(|change| change.text);
                if let (Some(document), Some(text)) =
                    (self.documents.get_mut(&params.text_document.uri), text)
                {
                    document.text = text;
                }
            }
            DidSaveTextDocument::METHOD => {
                let params: DidSaveTextDocumentParams =
                    notification.extract(DidSaveTextDocument::METHOD)?;
                self.analyze(&params.text_document.uri, false)?;
            }
            DidCloseTextDocument::METHOD => {
                let params: DidCloseTextDocumentParams =
                    notification.extract(DidCloseTextDocument::METHOD)?;
                let document = self.documents.remove(&params.text_document.uri);
                // Project findings stay listed for closed files, as in VS Code.
                let in_project = document
                    .and_then(|document| document.canonical)
                    .is_some_and(|file| self.projects.root_of(&file).is_some());
                if !in_project {
                    self.publish(params.text_document.uri, Vec::new())?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Analyzes the saved file behind an open document, together with the
    /// rest of its workspace folder when it is in one, and publishes the
    /// findings. With `reuse`, a folder analyzed before is not analyzed again.
    fn analyze(&mut self, uri: &Uri, reuse: bool) -> Result<(), BoxError> {
        let root = self
            .documents
            .get(uri)
            .and_then(|document| document.canonical.as_deref())
            .and_then(|file| self.projects.root_of(file))
            .map(Path::to_path_buf);
        if let Some(root) = root {
            return self.analyze_project(&root, uri, reuse);
        }
        let Some(document) = self.documents.get_mut(uri) else {
            return Ok(());
        };
        let Some(path) = &document.path else {
            return Ok(()); // Unsaved or remote buffers have no file to analyze.
        };
        match crate::analysis::analyze(path, &self.extra_args) {
            Ok(analysis) => {
                let found = entries(&analysis.report, &analysis.text);
                let diagnostics = found.iter().map(|entry| entry.diagnostic.clone()).collect();
                document.analysis = Some((analysis.text, found));
                self.publish(uri.clone(), diagnostics)
            }
            Err(message) => self.log_error(message),
        }
    }

    fn analyze_project(&mut self, root: &Path, uri: &Uri, reuse: bool) -> Result<(), BoxError> {
        let changed = if reuse && self.projects.results(root).is_some() {
            self.documents
                .get(uri)
                .and_then(|document| document.canonical.clone())
                .into_iter()
                .collect()
        } else {
            match self.projects.analyze(root, &self.extra_args) {
                Ok(changed) => changed,
                Err(message) => return self.log_error(message),
            }
        };
        let results = self.projects.results(root);
        // Code actions on open buffers use the analysis their text matches.
        for document in self.documents.values_mut() {
            let Some(file) = &document.canonical else {
                continue;
            };
            if !file.starts_with(root) {
                continue;
            }
            document.analysis = Some(match results.and_then(|files| files.get(file)) {
                Some(result) => (result.text.clone(), result.entries.clone()),
                None => (document.text.clone(), Vec::new()),
            });
        }
        for file in changed {
            let found = results.and_then(|files| files.get(&file));
            let diagnostics = found.map_or_else(Vec::new, |result| {
                result
                    .entries
                    .iter()
                    .map(|e| e.diagnostic.clone())
                    .collect()
            });
            let open = self
                .documents
                .iter()
                .find(|(_, document)| document.canonical.as_ref() == Some(&file))
                .map(|(uri, _)| uri.clone());
            if let Some(uri) = open.or_else(|| crate::uri::from_path(&file)) {
                self.publish(uri, diagnostics)?;
            }
        }
        Ok(())
    }

    fn log_error(&self, message: String) -> Result<(), BoxError> {
        let params = LogMessageParams {
            typ: MessageType::ERROR,
            message,
        };
        let log = Notification::new(LogMessage::METHOD.to_owned(), params);
        self.connection.sender.send(log.into())?;
        Ok(())
    }

    fn publish(&self, uri: Uri, diagnostics: Vec<lsp_types::Diagnostic>) -> Result<(), BoxError> {
        let params = PublishDiagnosticsParams {
            uri,
            diagnostics,
            version: None,
        };
        let notification = Notification::new(PublishDiagnostics::METHOD.to_owned(), params);
        self.connection.sender.send(notification.into())?;
        Ok(())
    }
}
