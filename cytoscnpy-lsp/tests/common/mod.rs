//! Helpers shared by the integration tests.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, dead_code)]

use std::time::Duration;

use lsp_server::{Connection, Message, Notification, Request, RequestId};
use serde_json::Value;

pub struct Client {
    pub connection: Connection,
    pub next_id: i32,
}

impl Client {
    pub fn request(&mut self, method: &str, params: Value) -> Value {
        self.next_id += 1;
        let id = RequestId::from(self.next_id);
        let request = Request::new(id.clone(), method.to_owned(), params);
        self.connection.sender.send(request.into()).unwrap();
        loop {
            match self.receive() {
                Message::Response(response) if response.id == id => {
                    return response.response_result.unwrap_or(Value::Null);
                }
                _ => {}
            }
        }
    }

    pub fn notify(&self, method: &str, params: Value) {
        let notification = Notification::new(method.to_owned(), params);
        self.connection.sender.send(notification.into()).unwrap();
    }

    /// The next `textDocument/publishDiagnostics` parameters.
    pub fn diagnostics(&self) -> Value {
        loop {
            if let Message::Notification(notification) = self.receive() {
                if notification.method == "textDocument/publishDiagnostics" {
                    return notification.params;
                }
                assert_ne!(notification.method, "window/logMessage", "{notification:?}");
            }
        }
    }

    pub fn receive(&self) -> Message {
        self.connection
            .receiver
            .recv_timeout(Duration::from_secs(60))
            .expect("the server should answer")
    }
}

pub fn file_uri(path: &std::path::Path) -> String {
    let uri = format!("file://{}", path.display()).replace('\\', "/");
    if uri.starts_with("file:///") {
        uri
    } else {
        uri.replacen("file://", "file:///", 1)
    }
}

pub fn start() -> (Client, std::thread::JoinHandle<()>) {
    let (server, client) = Connection::memory();
    let handle = std::thread::spawn(move || cytoscnpy_lsp::run(&server).unwrap());
    let client = Client {
        connection: client,
        next_id: 0,
    };
    (client, handle)
}

pub fn codes(params: &Value) -> Vec<&str> {
    let mut codes: Vec<_> = params["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .map(|diagnostic| diagnostic["code"].as_str().unwrap())
        .collect();
    codes.sort_unstable();
    codes
}
