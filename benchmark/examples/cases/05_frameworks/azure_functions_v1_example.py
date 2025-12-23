# Azure Functions v1 Example (Configuration-based)
# This file demonstrates Azure Functions v1 programming model
# Functions with the conventional 'main' name should be treated as framework entry points

import azure.functions as func
import logging


# Main entry point - called by Azure Functions runtime via function.json
# Should NOT be flagged as dead code
def main(req: func.HttpRequest) -> func.HttpResponse:
    logging.info("Python HTTP trigger function processed a request.")
    
    # Taint source: req.params.get()
    name = req.params.get("name")
    if not name:
        try:
            # Taint source: req.get_json()
            req_body = req.get_json()
        except ValueError:
            pass
        else:
            name = req_body.get("name")

    if name:
        processed_name = process_request_data(name)
        return func.HttpResponse(f"Hello, {processed_name}!")
    else:
        return func.HttpResponse(
            "Please pass a name on the query string or in the request body",
            status_code=400,
        )


# Helper function called by main - should NOT be flagged
def process_request_data(data):
    """Process and validate request data."""
    if not data:
        return None
    return data.strip().upper()


# DEAD CODE - This helper is NOT called and should be flagged
def unused_processor():
    return "This function is never called"
