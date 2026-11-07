//! Messages the server sends to the client, and what the client supports.

use std::collections::HashMap;
use std::error::Error;

use lsp_server::{Connection, Notification, Request, RequestId};
use lsp_types::notification::{LogMessage, Progress, PublishDiagnostics, ShowMessage};
use lsp_types::request::{
    RegisterCapability, ShowDocument, WorkDoneProgressCreate, WorkspaceConfiguration,
};
use lsp_types::{
    ClientCapabilities, ConfigurationItem, ConfigurationParams, Diagnostic,
    DidChangeWatchedFilesRegistrationOptions, FileSystemWatcher, GlobPattern, LogMessageParams,
    MessageType, NumberOrString, ProgressParams, ProgressParamsValue, PublishDiagnosticsParams,
    Registration, RegistrationParams, ShowDocumentParams, ShowMessageParams, Uri, WorkDoneProgress,
    WorkDoneProgressBegin, WorkDoneProgressCreateParams, WorkDoneProgressEnd,
};

pub(crate) type BoxError = Box<dyn Error + Send + Sync>;

/// What a response to one of the server's requests carries.
pub(crate) enum Pending {
    /// The `cytoscnpy` workspace configuration.
    Configuration,
    /// Nothing the server needs.
    Ignored,
}

pub(crate) struct Client<'a> {
    connection: &'a Connection,
    next_id: i32,
    progress: bool,
    show_document: bool,
    configuration: bool,
    watch_files: bool,
    /// Whether the current run of failed analyses has shown its popup.
    failure_shown: bool,
    /// The server's requests awaiting a response.
    pub pending: HashMap<RequestId, Pending>,
}

impl<'a> Client<'a> {
    pub(crate) fn new(connection: &'a Connection, capabilities: &ClientCapabilities) -> Self {
        let window = capabilities.window.as_ref();
        let workspace = capabilities.workspace.as_ref();
        Self {
            connection,
            next_id: 0,
            progress: window.and_then(|w| w.work_done_progress) == Some(true),
            show_document: window
                .and_then(|w| w.show_document.as_ref())
                .is_some_and(|show| show.support),
            configuration: workspace.and_then(|w| w.configuration) == Some(true),
            watch_files: workspace
                .and_then(|w| w.did_change_watched_files.as_ref())
                .and_then(|watch| watch.dynamic_registration)
                == Some(true),
            failure_shown: false,
            pending: HashMap::new(),
        }
    }

    pub(crate) fn send(&self, message: impl Into<lsp_server::Message>) -> Result<(), BoxError> {
        self.connection.sender.send(message.into())?;
        Ok(())
    }

    fn notify<N: lsp_types::notification::Notification>(
        &self,
        params: N::Params,
    ) -> Result<(), BoxError> {
        self.send(Notification::new(N::METHOD.to_owned(), params))
    }

    fn request<R: lsp_types::request::Request>(
        &mut self,
        params: R::Params,
        pending: Pending,
    ) -> Result<(), BoxError> {
        self.next_id += 1;
        let id = RequestId::from(format!("cytoscnpy-{}", self.next_id));
        self.pending.insert(id.clone(), pending);
        self.send(Request::new(id, R::METHOD.to_owned(), params))
    }

    pub(crate) fn publish(&self, uri: Uri, diagnostics: Vec<Diagnostic>) -> Result<(), BoxError> {
        self.notify::<PublishDiagnostics>(PublishDiagnosticsParams {
            uri,
            diagnostics,
            version: None,
        })
    }

    pub(crate) fn log(&self, typ: MessageType, message: String) -> Result<(), BoxError> {
        self.notify::<LogMessage>(LogMessageParams { typ, message })
    }

    pub(crate) fn show(&self, typ: MessageType, message: String) -> Result<(), BoxError> {
        self.notify::<ShowMessage>(ShowMessageParams { typ, message })
    }

    /// Logs a failed analysis, and shows it once per run of failures, as
    /// the VS Code extension does: a broken setup would otherwise raise a
    /// popup on every save.
    pub(crate) fn report_failure(&mut self, message: &str) -> Result<(), BoxError> {
        self.log(MessageType::ERROR, message.to_owned())?;
        if !self.failure_shown {
            self.failure_shown = true;
            self.show(
                MessageType::ERROR,
                format!("CytoScnPy analysis failed: {message}"),
            )?;
        }
        Ok(())
    }

    pub(crate) fn report_success(&mut self) {
        self.failure_shown = false;
    }

    /// Starts a progress report the client shows while analysis runs, like
    /// the VS Code extension's status bar item.
    pub(crate) fn progress_begin(&mut self, message: String) -> Result<Option<String>, BoxError> {
        if !self.progress {
            return Ok(None);
        }
        let token = format!("cytoscnpy-progress-{}", self.next_id + 1);
        let params = WorkDoneProgressCreateParams {
            token: NumberOrString::String(token.clone()),
        };
        self.request::<WorkDoneProgressCreate>(params, Pending::Ignored)?;
        let begin = WorkDoneProgressBegin {
            title: "CytoScnPy".to_owned(),
            message: Some(message),
            ..WorkDoneProgressBegin::default()
        };
        self.notify::<Progress>(ProgressParams {
            token: NumberOrString::String(token.clone()),
            value: ProgressParamsValue::WorkDone(WorkDoneProgress::Begin(begin)),
        })?;
        Ok(Some(token))
    }

    pub(crate) fn progress_end(
        &self,
        token: Option<String>,
        message: String,
    ) -> Result<(), BoxError> {
        let Some(token) = token else {
            return Ok(());
        };
        self.notify::<Progress>(ProgressParams {
            token: NumberOrString::String(token),
            value: ProgressParamsValue::WorkDone(WorkDoneProgress::End(WorkDoneProgressEnd {
                message: Some(message),
            })),
        })
    }

    /// Asks for the `cytoscnpy` settings, when the client serves them.
    pub(crate) fn request_configuration(&mut self) -> Result<bool, BoxError> {
        if !self.configuration {
            return Ok(false);
        }
        let params = ConfigurationParams {
            items: vec![ConfigurationItem {
                scope_uri: None,
                section: Some("cytoscnpy".to_owned()),
            }],
        };
        self.request::<WorkspaceConfiguration>(params, Pending::Configuration)?;
        Ok(true)
    }

    /// Asks to hear about Python and configuration files changing on disk,
    /// so changes made outside the editor are picked up, as the VS Code
    /// extension's periodic rescan does.
    pub(crate) fn watch_files(&mut self) -> Result<(), BoxError> {
        if !self.watch_files {
            return Ok(());
        }
        let watchers = ["**/*.py", "**/pyproject.toml", "**/.cytoscnpy.toml"]
            .into_iter()
            .map(|glob| FileSystemWatcher {
                glob_pattern: GlobPattern::String(glob.to_owned()),
                kind: None,
            })
            .collect();
        let options = DidChangeWatchedFilesRegistrationOptions { watchers };
        let registration = Registration {
            id: "cytoscnpy-watch".to_owned(),
            method: "workspace/didChangeWatchedFiles".to_owned(),
            register_options: Some(serde_json::to_value(options)?),
        };
        let params = RegistrationParams {
            registrations: vec![registration],
        };
        self.request::<RegisterCapability>(params, Pending::Ignored)
    }

    /// Shows `text` to the user: in an editor tab when the client can open
    /// documents, like the VS Code extension's output channel, and as a
    /// message otherwise.
    pub(crate) fn show_text(&mut self, name: &str, text: String) -> Result<(), BoxError> {
        if self.show_document {
            let dir = std::env::temp_dir().join("cytoscnpy");
            let file = dir.join(name);
            let written = std::fs::create_dir_all(&dir).and_then(|()| std::fs::write(&file, &text));
            if let (Ok(()), Some(uri)) = (written, crate::uri::from_path(&file)) {
                let params = ShowDocumentParams {
                    uri,
                    external: Some(false),
                    take_focus: Some(true),
                    selection: None,
                };
                return self.request::<ShowDocument>(params, Pending::Ignored);
            }
        }
        self.show(MessageType::INFO, text)
    }
}
