"""
Pattern Matching (PEP 634) usage patterns.
Tests detection of variables bound in case statements.
"""

def handle_command(command):
    match command:
        case "quit":
            print("Quitting")
        case ["load", filename]:
            # filename is bound and used
            print(f"Loading {filename}")
        case ["save", filename]:
            # filename is bound but NOT used -> DEAD
            print("Saving...")
        case {"x": x, "y": y}:
            # x and y bound
            # x used, y used
            print(f"Point at {x}, {y}")
        case _:
            print("Unknown command")

def unused_binding():
    data = [1, 2]
    match data:
        # a, b bound but unused -> DEAD
        case [a, b]:
            print("Two items")

def used_binding_in_guard():
    data = [1, 2]
    match data:
        case [x, y] if x > y:
            # x, y used in guard
            print("Descending")
