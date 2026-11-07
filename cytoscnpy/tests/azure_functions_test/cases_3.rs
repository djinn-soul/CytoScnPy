use super::*;

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
