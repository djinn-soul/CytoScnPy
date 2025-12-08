"""
Modern Python Features Example
Run with: cytoscnpy examples/modern_python.py
"""

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    # Imports inside TYPE_CHECKING blocks are treated with lower confidence/ignored
    from typing import List

def match_statement(status):
    # Python 3.10+ match statement support
    match status:
        case 200:
            return "OK"
        case 404:
            return "Not Found"
        case _:
            return "Unknown"

def type_hints(x: int) -> str:
    return str(x)

def main():
    print(match_statement(200))
    print(type_hints(10))

if __name__ == "__main__":
    main()
