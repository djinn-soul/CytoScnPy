"""
Dynamic Patterns Example
Run with: cytoscnpy examples/dynamic_patterns.py
"""

class MyClass:
    def dynamic_method(self):
        print("Called dynamically")

def dynamic_func():
    print("Called via globals")

def main():
    obj = MyClass()
    
    # Hasattr usage marks 'dynamic_method' as used
    if hasattr(obj, "dynamic_method"):
        getattr(obj, "dynamic_method")()

    # Globals usage marks 'dynamic_func' as used
    g = globals()
    g["dynamic_func"]()

if __name__ == "__main__":
    main()
