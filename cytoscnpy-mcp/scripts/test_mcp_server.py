"""
Test script for CytoScnPy MCP server.
Uses subprocess to communicate with the server via JSON-RPC over stdio.
"""

import subprocess
import json
import time


def test_mcp_server():
    """Test the MCP server by communicating via stdio."""
    
    # Start the MCP server process
    process = subprocess.Popen(
        [r"E:\Github\CytoScnPy\target\release\cytoscnpy-mcp.exe"],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    
    # Send initialize request
    init_request = {
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {"name": "test-client", "version": "1.0.0"},
        },
    }
    
    print("Sending initialize request...")
    request_bytes = (json.dumps(init_request) + "\n").encode('utf-8')
    process.stdin.write(request_bytes)
    process.stdin.flush()
    
    time.sleep(0.5)
    
    # Read response
    response = process.stdout.readline().decode('utf-8')
    print(f"Initialize response:\n{json.dumps(json.loads(response), indent=2)}")
    
    # Send initialized notification
    initialized_notif = {
        "jsonrpc": "2.0",
        "method": "notifications/initialized",
    }
    notif_bytes = (json.dumps(initialized_notif) + "\n").encode('utf-8')
    process.stdin.write(notif_bytes)
    process.stdin.flush()
    
    time.sleep(0.5)
    
    # Send tools/list request
    list_tools_request = {
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list",
        "params": {},
    }
    
    print("\nSending tools/list request...")
    request_bytes = (json.dumps(list_tools_request) + "\n").encode('utf-8')
    process.stdin.write(request_bytes)
    process.stdin.flush()
    
    time.sleep(0.5)
    
    # Read response
    response = process.stdout.readline().decode('utf-8')
    if response:
        print(f"Tools list response:\n{json.dumps(json.loads(response), indent=2)}")
    else:
        print("No response received for tools/list")
        # Check stderr
        stderr = process.stderr.read()
        if stderr:
            print(f"Stderr: {stderr.decode('utf-8')}")
    
    # Terminate the server
    process.terminate()
    print("\nTest complete!")


if __name__ == "__main__":
    test_mcp_server()
