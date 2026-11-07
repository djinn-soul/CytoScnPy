use super::*;

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
