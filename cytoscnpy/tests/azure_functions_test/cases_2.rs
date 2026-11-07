use super::*;

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
    let code = r"
import azure.functions as func

# No functions defined
app = func.FunctionApp()
";

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
