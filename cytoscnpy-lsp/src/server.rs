//! The LSP message loop: requests, notifications and debounced analysis.

use std::collections::HashMap;
use std::time::Duration;

use crossbeam_channel::RecvTimeoutError;
use lsp_server::{Connection, ErrorCode, Message, Notification, Request, Response};
use lsp_types::notification::{
    DidChangeConfiguration, DidChangeTextDocument, DidChangeWatchedFiles, DidCloseTextDocument,
    DidOpenTextDocument, DidSaveTextDocument, Notification as _,
};
use lsp_types::request::{CodeActionRequest, ExecuteCommand, Request as _};
use lsp_types::{
    CodeActionKind, CodeActionOptions, CodeActionParams, CodeActionProviderCapability,
    DidChangeConfigurationParams, DidChangeTextDocumentParams, DidChangeWatchedFilesParams,
    DidCloseTextDocumentParams, DidOpenTextDocumentParams, DidSaveTextDocumentParams,
    ExecuteCommandOptions, ExecuteCommandParams, InitializeParams, InitializeResult, SaveOptions,
    ServerCapabilities, ServerInfo, TextDocumentSyncCapability, TextDocumentSyncKind,
    TextDocumentSyncOptions, TextDocumentSyncSaveOptions,
};

use crate::actions::{code_actions, Context};
use crate::client::{BoxError, Client, Pending};
use crate::project::Projects;
use crate::settings::Settings;
use crate::workspace::{Document, Target};

/// How long the server waits for more saves before analyzing, so saving
/// several files at once runs one analysis.
const DEBOUNCE: Duration = Duration::from_millis(150);

pub(crate) struct Server<'a> {
    pub client: Client<'a>,
    pub documents: HashMap<lsp_types::Uri, Document>,
    pub init_settings: Settings,
    pub workspace_settings: Settings,
    pub projects: Projects,
    /// Analyses waiting for the debounce to pass.
    pub queue: Vec<Target>,
}

/// Serves LSP on `connection` until the client shuts the server down.
///
/// Settings named like the VS Code extension's (`enableSecretsScan`, `maxComplexity`, `args` and the rest)
/// come from the initialization options and the `cytoscnpy` workspace
/// configuration.
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
        client: Client::new(connection, &params.capabilities),
        documents: HashMap::new(),
        init_settings: Settings::from_value(params.initialization_options.as_ref()),
        workspace_settings: Settings::default(),
        projects: Projects::new(&params),
        queue: Vec::new(),
    };
    // `initialize_finish` consumed the client's `initialized` notification.
    server.client.watch_files()?;
    server.client.request_configuration()?;
    loop {
        let message = if server.queue.is_empty() {
            match connection.receiver.recv() {
                Ok(message) => message,
                Err(_) => break,
            }
        } else {
            match connection.receiver.recv_timeout(DEBOUNCE) {
                Ok(message) => message,
                Err(RecvTimeoutError::Timeout) => {
                    server.run_queue()?;
                    continue;
                }
                Err(RecvTimeoutError::Disconnected) => break,
            }
        };
        match message {
            Message::Request(request) => {
                if connection.handle_shutdown(&request)? {
                    return Ok(());
                }
                server.on_request(request)?;
            }
            Message::Notification(notification) => server.on_notification(notification)?,
            Message::Response(response) => server.on_response(response),
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
            code_action_kinds: Some(vec![CodeActionKind::QUICKFIX, CodeActionKind::SOURCE]),
            ..CodeActionOptions::default()
        })),
        execute_command_provider: Some(ExecuteCommandOptions {
            commands: crate::commands::COMMANDS
                .iter()
                .map(|(id, _)| (*id).to_owned())
                .collect(),
            ..ExecuteCommandOptions::default()
        }),
        ..ServerCapabilities::default()
    }
}

impl Server<'_> {
    pub(crate) fn settings(&self) -> Settings {
        self.init_settings.merged(&self.workspace_settings)
    }

    fn on_request(&mut self, request: Request) -> Result<(), BoxError> {
        let response = match request.method.as_str() {
            CodeActionRequest::METHOD => {
                let (id, params) =
                    request.extract::<CodeActionParams>(CodeActionRequest::METHOD)?;
                let uri = &params.text_document.uri;
                let mut actions = self
                    .documents
                    .get(uri)
                    .map(|document| {
                        let context = Context {
                            uri,
                            text: &document.text,
                            fresh_entries: document.fresh_entries(),
                        };
                        code_actions(&context, params.range, &params.context.diagnostics)
                    })
                    .unwrap_or_default();
                actions.extend(crate::commands::source_actions(uri, &params.context));
                Response::new_ok(id, actions)
            }
            ExecuteCommand::METHOD => {
                let (id, params) =
                    request.extract::<ExecuteCommandParams>(ExecuteCommand::METHOD)?;
                self.execute(&params)?;
                Response::new_ok(id, serde_json::Value::Null)
            }
            _ => {
                let message = format!("cytoscnpy does not handle {}", request.method);
                Response::new_err(request.id, ErrorCode::MethodNotFound as i32, message)
            }
        };
        self.client.send(response)
    }

    fn on_response(&mut self, response: Response) {
        if let Some(Pending::Configuration) = self.client.pending.remove(&response.id) {
            // The result holds one value per requested section.
            let value = response
                .response_result
                .ok()
                .and_then(|result| result.as_array().and_then(|items| items.first().cloned()));
            self.apply_workspace_settings(Settings::from_value(value.as_ref()));
        }
    }

    fn on_notification(&mut self, notification: Notification) -> Result<(), BoxError> {
        match notification.method.as_str() {
            DidOpenTextDocument::METHOD => {
                let params: DidOpenTextDocumentParams =
                    notification.extract(DidOpenTextDocument::METHOD)?;
                let uri = params.text_document.uri;
                let document = Document::new(&uri, params.text_document.text);
                self.documents.insert(uri.clone(), document);
                self.on_open(&uri)?;
            }
            DidChangeTextDocument::METHOD => {
                let params: DidChangeTextDocumentParams =
                    notification.extract(DidChangeTextDocument::METHOD)?;
                // Full sync: the last change holds the whole text.
                if let Some(change) = params.content_changes.into_iter().last() {
                    self.on_edit(&params.text_document.uri, change.text)?;
                }
            }
            DidSaveTextDocument::METHOD => {
                let params: DidSaveTextDocumentParams =
                    notification.extract(DidSaveTextDocument::METHOD)?;
                let target = self.target_of(&params.text_document.uri);
                self.enqueue(target);
            }
            DidCloseTextDocument::METHOD => {
                let params: DidCloseTextDocumentParams =
                    notification.extract(DidCloseTextDocument::METHOD)?;
                self.on_close(params.text_document.uri)?;
            }
            DidChangeConfiguration::METHOD => {
                let params: DidChangeConfigurationParams =
                    notification.extract(DidChangeConfiguration::METHOD)?;
                // Clients that serve configuration send no settings here.
                if !self.client.request_configuration()? {
                    self.apply_workspace_settings(Settings::from_value(Some(&params.settings)));
                }
            }
            DidChangeWatchedFiles::METHOD => {
                let params: DidChangeWatchedFilesParams =
                    notification.extract(DidChangeWatchedFiles::METHOD)?;
                self.on_files_changed(&params);
            }
            _ => {}
        }
        Ok(())
    }

    fn apply_workspace_settings(&mut self, settings: Settings) {
        if settings != self.workspace_settings {
            self.workspace_settings = settings;
            self.enqueue(Target::Workspace);
        }
    }
}
