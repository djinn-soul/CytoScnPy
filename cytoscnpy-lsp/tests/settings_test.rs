//! Settings, file watching and commands, as Zed and VS Code use them.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use common::{codes, file_uri, start, Client};
use lsp_server::{Message, Request, RequestId, Response};
use serde_json::{json, Value};

const BRANCHY: &str = "def branchy(x):\n    if x > 1:\n        if x > 2:\n            return 3\n        return 2\n    return 1\n\n\nbranchy(1)\n";

/// The next request the server sends, after answering it with `result`.
fn answer_next_request(client: &Client, result: &Value) -> Request {
    loop {
        if let Message::Request(request) = client.receive() {
            let response = Response::new_ok(request.id.clone(), result);
            client.connection.sender.send(response.into()).unwrap();
            return request;
        }
    }
}

#[test]
fn pulls_settings_watches_files_and_runs_commands() {
    let dir = tempfile::tempdir().unwrap();
    let project = dir.path().canonicalize().unwrap();
    let file = project.join("app.py");
    std::fs::write(&file, BRANCHY).unwrap();
    let uri = file_uri(&file);
    let (mut client, handle) = start();

    client.request(
        "initialize",
        json!({
            "capabilities": {"workspace": {
                "configuration": true,
                "didChangeWatchedFiles": {"dynamicRegistration": true}
            }},
            "workspaceFolders": [{"uri": file_uri(&project), "name": "p"}]
        }),
    );
    client.notify("initialized", json!({}));

    // Right after initializing, the server registers its file watchers and
    // asks for the `cytoscnpy` settings.
    let register = answer_next_request(&client, &Value::Null);
    assert_eq!(register.method, "client/registerCapability");
    assert_eq!(
        register.params["registrations"][0]["method"],
        "workspace/didChangeWatchedFiles"
    );
    let configuration = answer_next_request(&client, &json!([{"maxComplexity": 1}]));
    assert_eq!(configuration.method, "workspace/configuration");
    assert_eq!(configuration.params["items"][0]["section"], "cytoscnpy");

    client.notify(
        "textDocument/didOpen",
        json!({"textDocument": {"uri": uri, "languageId": "python", "version": 1, "text": BRANCHY}}),
    );
    // The threshold from the settings reports the nested branches.
    let published = client.diagnostics();
    assert_eq!(published["uri"], uri.as_str());
    assert!(codes(&published).contains(&"CSP-Q301"), "{published}");

    // Without `window/showDocument`, metrics come back as a message.
    client.next_id += 1;
    let id = RequestId::from(client.next_id);
    let execute = Request::new(
        id.clone(),
        "workspace/executeCommand".to_owned(),
        json!({"command": "cytoscnpy.complexity", "arguments": [uri]}),
    );
    client.connection.sender.send(execute.into()).unwrap();
    let mut shown = None;
    loop {
        match client.receive() {
            Message::Notification(notification) if notification.method == "window/showMessage" => {
                shown = Some(notification.params["message"].as_str().unwrap().to_owned());
            }
            Message::Response(response) if response.id == id => break,
            _ => {}
        }
    }
    let shown = shown.expect("the report should be shown");
    assert!(
        shown.starts_with("CytoScnPy Cyclomatic Complexity"),
        "{shown}"
    );
    assert!(shown.contains("branchy"), "{shown}");

    client.request("shutdown", Value::Null);
    client.notify("exit", Value::Null);
    handle.join().unwrap();
}

#[test]
fn shows_a_failed_analysis_once_as_a_prompt() {
    let dir = tempfile::tempdir().unwrap();
    let project = dir.path().canonicalize().unwrap();
    let file = project.join("app.py");
    std::fs::write(&file, BRANCHY).unwrap();
    let uri = file_uri(&file);
    let (mut client, handle) = start();

    client.request(
        "initialize",
        json!({
            "capabilities": {"window": {"showMessage": {}}},
            "initializationOptions": {"args": ["--bogus-flag"]},
            "workspaceFolders": [{"uri": file_uri(&project), "name": "p"}]
        }),
    );
    client.notify("initialized", json!({}));
    client.notify(
        "textDocument/didOpen",
        json!({"textDocument": {"uri": uri, "languageId": "python", "version": 1, "text": BRANCHY}}),
    );
    // Zed shows message requests; it only logs `window/showMessage`.
    let prompt = answer_next_request(&client, &Value::Null);
    assert_eq!(prompt.method, "window/showMessageRequest");
    let message = prompt.params["message"].as_str().unwrap();
    assert!(
        message.starts_with("CytoScnPy analysis failed"),
        "{message}"
    );

    // Failing again does not prompt again.
    client.notify(
        "textDocument/didSave",
        json!({"textDocument": {"uri": uri}}),
    );
    let id = RequestId::from(99);
    let actions = Request::new(
        id.clone(),
        "textDocument/codeAction".to_owned(),
        json!({"textDocument": {"uri": uri},
               "range": {"start": {"line": 0, "character": 0}, "end": {"line": 0, "character": 0}},
               "context": {"diagnostics": []}}),
    );
    // Sent after the debounce, so the second analysis runs first.
    std::thread::sleep(std::time::Duration::from_millis(500));
    client.connection.sender.send(actions.into()).unwrap();
    loop {
        match client.receive() {
            Message::Request(request) => panic!("unexpected {}", request.method),
            Message::Response(response) if response.id == id => break,
            _ => {}
        }
    }

    client.request("shutdown", Value::Null);
    client.notify("exit", Value::Null);
    handle.join().unwrap();
}
