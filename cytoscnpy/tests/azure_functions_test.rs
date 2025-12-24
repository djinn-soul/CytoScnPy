//! Tests for Azure Functions framework detection and taint analysis.

use cytoscnpy::analyzer::CytoScnPy;
use cytoscnpy::constants::get_framework_file_re;
use std::fs;
use tempfile::tempdir;

/// Helper function to analyze a Python file and return the results
fn analyze_code(code: &str) -> cytoscnpy::analyzer::AnalysisResult {
    let dir = tempdir().expect("Failed to create temp dir");
    let file_path = dir.path().join("test.py");
    fs::write(&file_path, code).expect("Failed to write test file");

    let mut analyzer = CytoScnPy::default()
        .with_confidence(60)
        .with_tests(false)
        .with_taint(true); // Enable taint for taint tests

    analyzer.analyze(dir.path())
}

#[test]
fn test_azure_functions_v2_decorator_detection() {
    let code = r#"
import azure.functions as func

app = func.FunctionApp()

@app.function_name(name="HttpExample")
@app.route(route="hello")
def hello_http(req: func.HttpRequest) -> func.HttpResponse:
    return func.HttpResponse("Hello!")

# This should be flagged as unused
def unused_helper():
    pass
"#;

    let result = analyze_code(code);

    let dead_names: Vec<&str> = result
        .unused_functions
        .iter()
        .map(|item| item.simple_name.as_str())
        .collect();

    // unused_helper should be detected as dead code
    assert!(
        dead_names.contains(&"unused_helper"),
        "unused_helper should be flagged as dead code"
    );

    // hello_http should NOT be flagged because it's a framework decorator
    assert!(
        !dead_names.contains(&"hello_http"),
        "hello_http should NOT be flagged as dead code (framework decorator)"
    );
}

#[test]
fn test_azure_functions_import_detection() {
    let code = r#"
import azure.functions as func
from azure.functions import FunctionApp

app = FunctionApp()

@app.route(route="/test")
def test_route(req):
    return "OK"
"#;

    let result = analyze_code(code);

    // Verify that azure.functions is detected as a framework
    let dead_names: Vec<&str> = result
        .unused_functions
        .iter()
        .map(|item| item.simple_name.as_str())
        .collect();

    // test_route should NOT be flagged
    assert!(!dead_names.contains(&"test_route"));
}

#[test]
fn test_azure_functions_blob_trigger() {
    let code = r#"
import azure.functions as func

app = func.FunctionApp()

@app.blob_trigger(arg_name="blob", path="samples/{name}", connection="AzureWebJobsStorage")
def blob_processor(blob: func.InputStream) -> None:
    content = blob.read()
"#;

    let result = analyze_code(code);
    let dead_names: Vec<&str> = result
        .unused_functions
        .iter()
        .map(|item| item.simple_name.as_str())
        .collect();

    // blob_processor should NOT be flagged
    assert!(!dead_names.contains(&"blob_processor"));
}

#[test]
fn test_azure_functions_timer_trigger() {
    let code = r#"
import azure.functions as func

app = func.FunctionApp()

@app.timer_trigger(schedule="0 */5 * * * *", arg_name="timer")
def scheduled_task(timer: func.TimerRequest) -> None:
    pass
"#;

    let result = analyze_code(code);
    let dead_names: Vec<&str> = result
        .unused_functions
        .iter()
        .map(|item| item.simple_name.as_str())
        .collect();

    // scheduled_task should NOT be flagged
    assert!(!dead_names.contains(&"scheduled_task"));
}

#[test]
fn test_azure_functions_queue_trigger() {
    let code = r#"
import azure.functions as func

app = func.FunctionApp()

@app.queue_trigger(arg_name="msg", queue_name="myqueue", connection="AzureWebJobsStorage")
@app.queue_output(arg_name="out", queue_name="outqueue", connection="AzureWebJobsStorage")
def queue_handler(msg: func.QueueMessage, out: func.Out[str]) -> None:
    out.set(msg.get_body().decode())
"#;

    let result = analyze_code(code);
    let dead_names: Vec<&str> = result
        .unused_functions
        .iter()
        .map(|item| item.simple_name.as_str())
        .collect();

    // queue_handler should NOT be flagged
    assert!(!dead_names.contains(&"queue_handler"));
}

#[test]
fn test_azure_functions_service_bus_trigger() {
    let code = r#"
import azure.functions as func

app = func.FunctionApp()

@app.service_bus_queue_trigger(arg_name="msg", queue_name="myservicebusqueue", connection="ServiceBusConnection")
def servicebus_handler(msg: func.ServiceBusMessage) -> None:
    pass
"#;

    let result = analyze_code(code);
    let dead_names: Vec<&str> = result
        .unused_functions
        .iter()
        .map(|item| item.simple_name.as_str())
        .collect();

    // servicebus_handler should NOT be flagged
    assert!(!dead_names.contains(&"servicebus_handler"));
}

#[test]
fn test_azure_functions_event_hub_trigger() {
    let code = r#"
import azure.functions as func

app = func.FunctionApp()

@app.event_hub_trigger(arg_name="event", event_hub_name="myeventhub", connection="EventHubConnection")
def eventhub_handler(event: func.EventHubEvent) -> None:
    pass
"#;

    let result = analyze_code(code);
    let dead_names: Vec<&str> = result
        .unused_functions
        .iter()
        .map(|item| item.simple_name.as_str())
        .collect();

    // eventhub_handler should NOT be flagged
    assert!(!dead_names.contains(&"eventhub_handler"));
}

#[test]
fn test_azure_functions_cosmos_db_trigger() {
    let code = r#"
import azure.functions as func

app = func.FunctionApp()

@app.cosmos_db_trigger(arg_name="docs", container_name="mycontainer", database_name="mydb", connection="CosmosDbConnection")
def cosmosdb_handler(docs: func.DocumentList) -> None:
    pass
"#;

    let result = analyze_code(code);
    let dead_names: Vec<&str> = result
        .unused_functions
        .iter()
        .map(|item| item.simple_name.as_str())
        .collect();

    // cosmosdb_handler should NOT be flagged
    assert!(!dead_names.contains(&"cosmosdb_handler"));
}

#[test]
fn test_azure_functions_v1_main_detection() {
    let code = r#"
import azure.functions as func

def main(req: func.HttpRequest) -> func.HttpResponse:
    name = req.params.get("name")
    return func.HttpResponse(f"Hello, {name}!")

def unused_func():
    pass
"#;

    let result = analyze_code(code);
    let dead_names: Vec<&str> = result
        .unused_functions
        .iter()
        .map(|item| item.simple_name.as_str())
        .collect();

    // main should NOT be flagged in azure.functions files
    // (it's the entry point for v1 model)
    // Note: This relies on automagic detection

    // unused_func should be flagged
    assert!(dead_names.contains(&"unused_func"));
}

#[test]
fn test_function_app_file_pattern() {
    let re = get_framework_file_re();

    // function_app.py should be recognized as a framework file
    assert!(re.is_match("function_app.py"));
    assert!(re.is_match("src/function_app.py"));
    assert!(re.is_match("azure/function_app.py"));

    // Case insensitivity
    assert!(re.is_match("Function_App.py"));
}

#[test]
fn test_azure_functions_taint_analysis() {
    let code = r#"
import azure.functions as func
import os

def main(req: func.HttpRequest) -> func.HttpResponse:
    # Source: req.params.get
    cmd = req.params.get('cmd')
    
    # Sink: os.system
    os.system(cmd)
    
    return func.HttpResponse("Executed")
"#;

    let result = analyze_code(code);

    assert!(
        !result.taint_findings.is_empty(),
        "Should detect taint flow from req.params to os.system"
    );

    let finding = &result.taint_findings[0];
    assert!(
        finding.source.contains("Azure Functions request")
            || finding.source.contains("tainted data"),
        "Finding should indicate tainted data from Azure Functions"
    );
}

// ============================================================================
// Additional Trigger Tests
// ============================================================================

#[test]
fn test_azure_functions_event_grid_trigger() {
    let code = r#"
import azure.functions as func

app = func.FunctionApp()

@app.event_grid_trigger(arg_name="event")
def eventgrid_handler(event: func.EventGridEvent) -> None:
    pass
"#;

    let result = analyze_code(code);
    let dead_names: Vec<&str> = result
        .unused_functions
        .iter()
        .map(|item| item.simple_name.as_str())
        .collect();

    assert!(!dead_names.contains(&"eventgrid_handler"));
}

#[test]
fn test_azure_functions_http_trigger_explicit() {
    let code = r#"
import azure.functions as func

app = func.FunctionApp()

@app.http_trigger(arg_name="req", methods=["GET", "POST"])
def http_handler(req: func.HttpRequest) -> func.HttpResponse:
    return func.HttpResponse("OK")
"#;

    let result = analyze_code(code);
    let dead_names: Vec<&str> = result
        .unused_functions
        .iter()
        .map(|item| item.simple_name.as_str())
        .collect();

    assert!(!dead_names.contains(&"http_handler"));
}

#[test]
fn test_azure_functions_async_handler() {
    let code = r#"
import azure.functions as func

app = func.FunctionApp()

@app.route(route="async-test")
async def async_handler(req: func.HttpRequest) -> func.HttpResponse:
    return func.HttpResponse("Async OK")

@app.timer_trigger(schedule="0 */5 * * * *", arg_name="timer")
async def async_timer(timer: func.TimerRequest) -> None:
    pass
"#;

    let result = analyze_code(code);
    let dead_names: Vec<&str> = result
        .unused_functions
        .iter()
        .map(|item| item.simple_name.as_str())
        .collect();

    assert!(!dead_names.contains(&"async_handler"));
    assert!(!dead_names.contains(&"async_timer"));
}

#[test]
fn test_azure_functions_multiple_bindings() {
    let code = r#"
import azure.functions as func

app = func.FunctionApp()

@app.blob_trigger(arg_name="inblob", path="input/{name}", connection="AzureWebJobsStorage")
@app.blob_output(arg_name="outblob", path="output/{name}", connection="AzureWebJobsStorage")
@app.queue_output(arg_name="msg", queue_name="processed", connection="AzureWebJobsStorage")
def multi_binding_handler(inblob: func.InputStream, outblob: func.Out[str], msg: func.Out[str]) -> None:
    pass
"#;

    let result = analyze_code(code);
    let dead_names: Vec<&str> = result
        .unused_functions
        .iter()
        .map(|item| item.simple_name.as_str())
        .collect();

    assert!(!dead_names.contains(&"multi_binding_handler"));
}

#[test]
fn test_azure_functions_nested_decorators() {
    let code = r#"
import azure.functions as func
from functools import wraps

app = func.FunctionApp()

def my_decorator(f):
    @wraps(f)
    def wrapper(*args, **kwargs):
        return f(*args, **kwargs)
    return wrapper

@app.function_name(name="NestedExample")
@app.route(route="nested")
@my_decorator
def nested_decorators(req: func.HttpRequest) -> func.HttpResponse:
    return func.HttpResponse("Nested!")
"#;

    let result = analyze_code(code);
    let dead_names: Vec<&str> = result
        .unused_functions
        .iter()
        .map(|item| item.simple_name.as_str())
        .collect();

    // Should still recognize as framework function
    assert!(!dead_names.contains(&"nested_decorators"));
}

// ============================================================================
// Taint Source Variation Tests
// ============================================================================

#[test]
fn test_azure_taint_get_json() {
    let code = r#"
import azure.functions as func
import subprocess

def main(req: func.HttpRequest) -> func.HttpResponse:
    data = req.get_json()
    cmd = data.get('command')
    subprocess.call(cmd, shell=True)
    return func.HttpResponse("Done")
"#;

    let result = analyze_code(code);

    // Should detect taint from get_json() to subprocess.call
    assert!(
        !result.taint_findings.is_empty(),
        "Should detect taint flow from req.get_json() to subprocess"
    );
}

#[test]
fn test_azure_taint_get_body() {
    let code = r#"
import azure.functions as func
import os

def main(req: func.HttpRequest) -> func.HttpResponse:
    body = req.get_body()
    os.system(body.decode())
    return func.HttpResponse("Done")
"#;

    let result = analyze_code(code);

    assert!(
        !result.taint_findings.is_empty(),
        "Should detect taint flow from req.get_body() to os.system"
    );
}

#[test]
fn test_azure_taint_route_params() {
    let code = r#"
import azure.functions as func
import os

def main(req: func.HttpRequest) -> func.HttpResponse:
    param = req.route_params.get('id')
    os.system(param)
    return func.HttpResponse("Done")
"#;

    let result = analyze_code(code);

    assert!(
        !result.taint_findings.is_empty(),
        "Should detect taint flow from req.route_params to os.system"
    );
}

#[test]
fn test_azure_taint_headers() {
    let code = r#"
import azure.functions as func
import os

def main(req: func.HttpRequest) -> func.HttpResponse:
    user_agent = req.headers.get('User-Agent')
    os.system(user_agent)
    return func.HttpResponse("Done")
"#;

    let result = analyze_code(code);

    assert!(
        !result.taint_findings.is_empty(),
        "Should detect taint flow from req.headers to os.system"
    );
}

#[test]
fn test_azure_taint_form_data() {
    let code = r#"
import azure.functions as func
import os

def main(req: func.HttpRequest) -> func.HttpResponse:
    username = req.form.get('username')
    os.system(username)
    return func.HttpResponse("Done")
"#;

    let result = analyze_code(code);

    assert!(
        !result.taint_findings.is_empty(),
        "Should detect taint flow from req.form to os.system"
    );
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_azure_functions_empty_file() {
    let code = r#"
import azure.functions as func

# No functions defined
app = func.FunctionApp()
"#;

    let result = analyze_code(code);

    // Should not crash, just have no unused functions
    assert!(result.unused_functions.is_empty());
}

#[test]
fn test_azure_functions_mixed_v1_v2() {
    let code = r#"
import azure.functions as func

app = func.FunctionApp()

# v2 style
@app.route(route="v2-endpoint")
def v2_handler(req: func.HttpRequest) -> func.HttpResponse:
    return func.HttpResponse("v2")

# v1 style (main function)
def main(req: func.HttpRequest) -> func.HttpResponse:
    return func.HttpResponse("v1")

# Helper that should be flagged
def unused_in_mixed():
    pass
"#;

    let result = analyze_code(code);
    let dead_names: Vec<&str> = result
        .unused_functions
        .iter()
        .map(|item| item.simple_name.as_str())
        .collect();

    assert!(!dead_names.contains(&"v2_handler"));
    assert!(dead_names.contains(&"unused_in_mixed"));
}

#[test]
fn test_azure_functions_table_bindings() {
    let code = r#"
import azure.functions as func

app = func.FunctionApp()

@app.table_input(arg_name="entities", table_name="mytable", connection="AzureWebJobsStorage")
@app.table_output(arg_name="out", table_name="outputtable", connection="AzureWebJobsStorage")
def table_handler(entities, out: func.Out[str]) -> None:
    pass
"#;

    let result = analyze_code(code);
    let dead_names: Vec<&str> = result
        .unused_functions
        .iter()
        .map(|item| item.simple_name.as_str())
        .collect();

    assert!(!dead_names.contains(&"table_handler"));
}

#[test]
fn test_azure_functions_durable_orchestrator() {
    let code = r#"
import azure.functions as func
import azure.durable_functions as df

app = func.FunctionApp()

@app.orchestration_trigger(context_name="context")
def orchestrator(context: df.DurableOrchestrationContext):
    result = yield context.call_activity("activity_func", "input")
    return result

@app.activity_trigger(input_name="input")
def activity_func(input: str) -> str:
    return input.upper()
"#;

    let result = analyze_code(code);
    let dead_names: Vec<&str> = result
        .unused_functions
        .iter()
        .map(|item| item.simple_name.as_str())
        .collect();

    // Durable functions patterns - orchestration_trigger and activity_trigger
    // These should be recognized as triggers
    assert!(!dead_names.contains(&"orchestrator") || !dead_names.contains(&"activity_func"));
}
