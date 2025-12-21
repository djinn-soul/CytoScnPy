"""
Test script for CytoScnPy MCP server.
Uses subprocess to communicate with the server via JSON-RPC over stdio.

Usage:
    python test_mcp_server.py
"""

import subprocess
import json
import time
import os


def send_request(process, request):
    """Send a JSON-RPC request and return the response."""
    request_bytes = (json.dumps(request) + "\n").encode('utf-8')
    process.stdin.write(request_bytes)
    process.stdin.flush()
    time.sleep(0.5)
    response = process.stdout.readline().decode('utf-8')
    return json.loads(response) if response else None


def test_mcp_server():
    """Test the MCP server by communicating via stdio."""
    
    # Find the CLI executable
    script_dir = os.path.dirname(os.path.abspath(__file__))
    repo_root = os.path.dirname(os.path.dirname(script_dir))
    
    # Try release first, then debug
    cli_path = os.path.join(repo_root, "target", "release", "cytoscnpy-cli.exe")
    if not os.path.exists(cli_path):
        cli_path = os.path.join(repo_root, "target", "debug", "cytoscnpy-cli.exe")
    
    if not os.path.exists(cli_path):
        print(f"Error: Could not find cytoscnpy-cli.exe")
        print(f"Looked in: {cli_path}")
        return
    
    print(f"Using CLI: {cli_path}")
    
    # Start the MCP server process using CLI subcommand
    process = subprocess.Popen(
        [cli_path, "mcp-server"],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    
    try:
        # Test 1: Initialize
        print("\n--- Test 1: Initialize ---")
        init_response = send_request(process, {
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {"name": "test-client", "version": "1.0.0"},
            },
        })
        
        if init_response:
            print(f"[PASS] Initialize response received")
            print(f"  Server: {init_response.get('result', {}).get('serverInfo', {})}")
        else:
            print("[FAIL] No initialize response")
            return
        
        # Send initialized notification
        process.stdin.write((json.dumps({
            "jsonrpc": "2.0",
            "method": "notifications/initialized",
        }) + "\n").encode('utf-8'))
        process.stdin.flush()
        time.sleep(0.3)
        
        # Test 2: List tools
        print("\n--- Test 2: List Tools ---")
        tools_response = send_request(process, {
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list",
            "params": {},
        })
        
        if tools_response:
            tools = tools_response.get('result', {}).get('tools', [])
            print(f"[PASS] Found {len(tools)} tools:")
            for tool in tools:
                print(f"  - {tool.get('name')}: {tool.get('description', '')[:60]}...")
        else:
            print("[FAIL] No tools/list response")
        
        # Test 3: Call analyze_code tool
        print("\n--- Test 3: Call analyze_code ---")
        analyze_response = send_request(process, {
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "analyze_code",
                "arguments": {
                    "code": "def unused_function():\n    pass\n\nimport os\nimport sys\n",
                    "filename": "test.py"
                }
            },
        })
        
        if analyze_response:
            print("[PASS] analyze_code response received")
            result = analyze_response.get('result', {})
            content = result.get('content', [])
            if content:
                text = content[0].get('text', '')
                data = json.loads(text)
                print(f"  Unused functions: {len(data.get('unused_functions', []))}")
                print(f"  Unused imports: {len(data.get('unused_imports', []))}")
        else:
            print("[FAIL] No analyze_code response")
        
        print("\n[PASS] All tests completed!")
        
    finally:
        process.terminate()
        process.wait()


if __name__ == "__main__":
    test_mcp_server()
