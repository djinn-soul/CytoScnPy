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

fn file_uri(path: &std::path::Path) -> String {
    let uri = format!("file://{}", path.display()).replace('\\', "/");
    if uri.starts_with("file:///") {
        uri
    } else {
        uri.replacen("file://", "file:///", 1)
    }
}

fn start() -> (Client, std::thread::JoinHandle<()>) {
    let (server, client) = Connection::memory();
    let handle = std::thread::spawn(move || cytoscnpy_lsp::run(&server).unwrap());
    let client = Client {
        connection: client,
        next_id: 0,
    };
    (client, handle)
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
    let uri = file_uri(&file);
    let (mut client, handle) = start();

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

#[test]
fn analyzes_the_workspace_folder_so_cross_file_uses_count() {
    let dir = tempfile::tempdir().unwrap();
    let project = dir.path().canonicalize().unwrap();
    let lib = project.join("lib.py");
    let lib_source = "def helper():\n    return 1\n\n\ndef orphan():\n    return 2\n";
    std::fs::write(&lib, lib_source).unwrap();
    let app = project.join("app.py");
    std::fs::write(
        &app,
        "import os\nfrom lib import helper\n\nprint(helper())\n",
    )
    .unwrap();
    let lib_uri = file_uri(&lib);
    let (mut client, handle) = start();

    client.request(
        "initialize",
        json!({"capabilities": {}, "workspaceFolders": [{"uri": file_uri(&project), "name": "p"}]}),
    );
    client.notify("initialized", json!({}));
    client.notify(
        "textDocument/didOpen",
        json!({"textDocument": {"uri": lib_uri, "languageId": "python", "version": 1, "text": lib_source}}),
    );
    // One analysis publishes every file with findings, open or not.
    let mut published = std::collections::HashMap::new();
    for _ in 0..2 {
        let params = client.diagnostics();
        let uri = params["uri"].as_str().unwrap().to_owned();
        let names: Vec<String> = params["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .map(|d| d["message"].as_str().unwrap().to_owned())
            .collect();
        published.insert(uri, names);
    }
    // `helper` is called from app.py, so only `orphan` is unused.
    assert_eq!(published[&lib_uri], ["'orphan' is defined but never used"]);
    assert_eq!(
        published[&file_uri(&app)],
        ["'os' is imported but never used"]
    );

    // Fixing app.py on disk and saving lib.py clears app.py's findings.
    std::fs::write(&app, "from lib import helper\n\nprint(helper())\n").unwrap();
    client.notify(
        "textDocument/didSave",
        json!({"textDocument": {"uri": lib_uri}}),
    );
    let mut cleared = false;
    for _ in 0..2 {
        let params = client.diagnostics();
        if params["uri"] == file_uri(&app).as_str() {
            cleared = params["diagnostics"].as_array().unwrap().is_empty();
        }
    }
    assert!(cleared);

    client.request("shutdown", Value::Null);
    client.notify("exit", Value::Null);
    handle.join().unwrap();
}
