//! Drives the server over an in-memory connection on real files.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::time::Duration;

use lsp_server::{Connection, Message, Notification, Request, RequestId};
use serde_json::{json, Value};

const SOURCE: &str = "import os\nimport sys\n\nprint(sys.argv)\neval(\"1\")\n";

struct Client {
    connection: Connection,
    next_id: i32,
}

impl Client {
    fn request(&mut self, method: &str, params: Value) -> Value {
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

    fn notify(&self, method: &str, params: Value) {
        let notification = Notification::new(method.to_owned(), params);
        self.connection.sender.send(notification.into()).unwrap();
    }

    /// The next `textDocument/publishDiagnostics` parameters.
    fn diagnostics(&self) -> Value {
        loop {
            if let Message::Notification(notification) = self.receive() {
                if notification.method == "textDocument/publishDiagnostics" {
                    return notification.params;
                }
                assert_ne!(notification.method, "window/logMessage", "{notification:?}");
            }
        }
    }

    fn receive(&self) -> Message {
        self.connection
            .receiver
            .recv_timeout(Duration::from_secs(60))
            .expect("the server should answer")
    }
}

fn codes(params: &Value) -> Vec<&str> {
    let mut codes: Vec<_> = params["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .map(|diagnostic| diagnostic["code"].as_str().unwrap())
        .collect();
    codes.sort_unstable();
    codes
}

#[test]
fn publishes_on_open_and_save_and_offers_fixes() {
    let dir = tempfile::tempdir().unwrap();
    let project = dir.path();
    std::fs::write(
        project.join("pyproject.toml"),
        "[tool.cytoscnpy]\ndanger = true\n",
    )
    .unwrap();
    let file = project.join("app.py");
    std::fs::write(&file, SOURCE).unwrap();
    let uri = format!("file://{}", file.display()).replace('\\', "/");
    let uri = if uri.starts_with("file:///") {
        uri
    } else {
        uri.replacen("file://", "file:///", 1)
    };

    let (server, client) = Connection::memory();
    let handle = std::thread::spawn(move || cytoscnpy_lsp::run(&server).unwrap());
    let mut client = Client {
        connection: client,
        next_id: 0,
    };

    let init = client.request("initialize", json!({"capabilities": {}}));
    assert_eq!(init["serverInfo"]["name"], "cytoscnpy");
    client.notify("initialized", json!({}));

    client.notify(
        "textDocument/didOpen",
        json!({"textDocument": {"uri": uri, "languageId": "python", "version": 1, "text": SOURCE}}),
    );
    let published = client.diagnostics();
    assert_eq!(published["uri"], uri.as_str());
    assert_eq!(codes(&published), ["CSP-D001", "unused-import"]);

    let unused = published["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .find(|diagnostic| diagnostic["code"] == "unused-import")
        .unwrap()
        .clone();
    let actions = client.request(
        "textDocument/codeAction",
        json!({"textDocument": {"uri": uri}, "range": unused["range"],
               "context": {"diagnostics": [unused]}}),
    );
    let titles: Vec<_> = actions
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["title"].clone())
        .collect();
    assert_eq!(
        titles,
        [
            json!("Remove unused import 'os'"),
            json!("Suppress with # noqa: unused-import")
        ]
    );

    // Saving the fixed file clears the import finding.
    let fixed = SOURCE.replacen("import os\n", "", 1);
    std::fs::write(&file, &fixed).unwrap();
    client.notify(
        "textDocument/didChange",
        json!({"textDocument": {"uri": uri, "version": 2}, "contentChanges": [{"text": fixed}]}),
    );
    client.notify(
        "textDocument/didSave",
        json!({"textDocument": {"uri": uri}}),
    );
    assert_eq!(codes(&client.diagnostics()), ["CSP-D001"]);

    client.notify(
        "textDocument/didClose",
        json!({"textDocument": {"uri": uri}}),
    );
    assert_eq!(codes(&client.diagnostics()), Vec::<&str>::new());

    client.request("shutdown", Value::Null);
    client.notify("exit", Value::Null);
    handle.join().unwrap();
}
