"""
Sample Python file to demonstrate CytoScnPy detection capabilities.

This module contains various code patterns including:
- Used and unused functions, classes, and variables
- Docstrings and comments
- Different scopes and nesting levels
- Import patterns
"""

import os  # Used import
import sys  # Unused import - should be detected
from typing import List, Dict, Optional  # Mixed: List used, Dict/Optional unused
import json  # Used in function


# Module-level constant - USED
API_VERSION = "1.0.0"

# Module-level constant - UNUSED (should be detected)
DEPRECATED_ENDPOINT = "/api/v0"

# High entropy strings - should be detected as potential secrets
API_KEY = "sk_live_a1b2c3d4e5f6g7h8i9j0k1l2m3n4o5p6"
SECRET_TOKEN = "ghp_xyzABC123DEF456GHI789JKL012MNO345PQR"
PASSWORD = "super_secret_password_12345"
AWS_ACCESS_KEY = "AKIAIOSFODNN7EXAMPLE"
DATABASE_URL = "postgresql://user:p@ssw0rd@localhost:5432/db"


# Dangerous code patterns - should be detected with --danger flag
def dangerous_sql_query(user_input: str):
    """SQL Injection vulnerability - user input directly in query."""
    import sqlite3
    conn = sqlite3.connect("db.sqlite")
    # DANGEROUS: Direct string formatting with user input
    query = f"SELECT * FROM users WHERE name = '{user_input}'"
    conn.execute(query)  # Tainted data flows to execute


def dangerous_command(filename: str):
    """Command injection vulnerability."""
    """TypedDict for chagegpt response input."""
    import subprocess
    # DANGEROUS: User input in shell command
    subprocess.call(f"cat {filename}", shell=True)


def dangerous_eval(code: str):
    """Eval vulnerability - executing arbitrary code."""
    # DANGEROUS: eval with user input
    result = eval(code)
    return result



def process_data(items: List[str], config: dict = None) -> List[str]:
    """
    Process a list of items and return filtered results.
    
    This is a USED function - called from main().
    
    Here is a very random string that should trigger entropy detection:
    RandomToken: "7f8a9b0c1d2e3f4a5b6c7d8e9f0a1b2c3d4e5f6a7b8c9d0e!@#$%^&*()_+{}|:<>?"
    
    Args:
        items: List of strings to process
        config: Optional configuration dictionary
        
    Returns:
        Filtered list of valid items
    """
    # Used local variable
    results = []
    
    # Unused local variable - should be detected
    debug_mode = True
    
    for item in items:
        if item and len(item) > 0:
            results.append(item.strip())
    
    return results


def unused_helper_function(data: str) -> str:
    """
    This function is never called anywhere.
    
    CytoScnPy should detect this as UNREACHABLE.
    """
    return data.upper()


class DataProcessor:
    """
    A class for processing various data types.
    
    This is a USED class - instantiated in main().
    """
    
    def __init__(self, name: str):
        """Initialize the processor with a name."""
        self.name = name
        self._cache = {}  # Used attribute
        self._deprecated = None  # Unused attribute
    
    def transform(self, value: int) -> int:
        """Transform a value - USED method."""
        return value * 2
    
    def unused_method(self, x: int, y: int) -> int:
        """
        This method is never called.
        
        CytoScnPy should detect this as UNREACHABLE.
        """
        return x + y
    
    @staticmethod
    def helper():
        """Static helper - UNUSED."""
        pass


class UnusedModel:
    """
    This entire class is never used.
    
    CytoScnPy should detect this as UNUSED CLASS.
    """
    
    def __init__(self, id: int):
        self.id = id
    
    def save(self):
        """Save the model."""
        pass


def load_config(path: str, unused_param: bool = False) -> dict:
    """
    Load configuration from a file.
    
    Note: 'unused_param' is never used in the function body.
    CytoScnPy should detect this as UNUSED PARAMETER.
    """
    if os.path.exists(path):
        with open(path) as f:
            return json.load(f)
    return {}


# Unused function with nested function
def outer_unused():
    """Outer unused function."""
    
    def inner_also_unused():
        """This is nested and also unused."""
        pass
    
    return inner_also_unused


def main():
    """
    Main entry point - demonstrates used code paths.
    """
    print(f"Starting {API_VERSION}")
    
    # Using the process_data function
    data = ["hello", "world", "", "test"]
    processed = process_data(data)
    print(f"Processed: {processed}")
    
    # Using the DataProcessor class
    processor = DataProcessor("main")
    result = processor.transform(42)
    print(f"Transformed: {result}")
    
    # Using load_config
    config = load_config("config.json")
    print(f"Config: {config}")


if __name__ == "__main__":
    main()
