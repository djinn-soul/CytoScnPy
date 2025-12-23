# Azure Functions v2 Example (Decorator-based)
# This file demonstrates Azure Functions v2 programming model
# Functions decorated with Azure Functions decorators should NOT be flagged as dead code

import azure.functions as func

app = func.FunctionApp()


# HTTP trigger - used by framework (should NOT be flagged as dead code)
@app.function_name(name="HttpExample")
@app.route(route="hello")
def hello_http(req: func.HttpRequest) -> func.HttpResponse:
    name = req.params.get("name")
    if not name:
        body = req.get_json()
        name = body.get("name")
    return func.HttpResponse(f"Hello, {name}!")


# Timer trigger - used by framework
@app.timer_trigger(schedule="0 */5 * * * *", arg_name="timer")
def timer_trigger(timer: func.TimerRequest) -> None:
    """Runs every 5 minutes."""
    pass


# Blob trigger with output binding
@app.blob_trigger(arg_name="blob", path="samples/{name}", connection="AzureWebJobsStorage")
@app.blob_output(
    arg_name="outputblob", path="output/{name}", connection="AzureWebJobsStorage"
)
def blob_trigger(blob: func.InputStream, outputblob: func.Out[str]) -> None:
    content = blob.read().decode("utf-8")
    outputblob.set(content.upper())


# Queue trigger
@app.queue_trigger(arg_name="msg", queue_name="myqueue", connection="AzureWebJobsStorage")
def queue_trigger(msg: func.QueueMessage) -> None:
    message_body = msg.get_body().decode("utf-8")
    pass


# Service Bus trigger
@app.service_bus_queue_trigger(
    arg_name="msg", queue_name="myservicebusqueue", connection="ServiceBusConnection"
)
def servicebus_trigger(msg: func.ServiceBusMessage) -> None:
    pass


# Event Hub trigger
@app.event_hub_trigger(arg_name="event", event_hub_name="myeventhub", connection="EventHubConnection")
def eventhub_trigger(event: func.EventHubEvent) -> None:
    pass


# Cosmos DB trigger
@app.cosmos_db_trigger(
    arg_name="docs",
    container_name="mycontainer",
    database_name="mydb",
    connection="CosmosDbConnection",
)
def cosmosdb_trigger(docs: func.DocumentList) -> None:
    pass


# DEAD CODE - This helper function is NOT called and should be flagged
def unused_helper():
    return "This should be flagged as dead code"


# DEAD CODE - Another unused function
def another_unused_function(x, y):
    return x + y
