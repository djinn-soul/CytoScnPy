"""
Basic Dead Code Detection Example
Run with: cytoscnpy examples/basic_dead_code.py
"""

import os
import sys  # Unused import

def used_function():
    print("I am used")

def unused_function():
    print("I am unused")

class UsedClass:
    def used_method(self):
        print("Used method")

    def unused_method(self):
        print("Unused method")

class UnusedClass:
    pass

def main():
    used_function()
    obj = UsedClass()
    obj.used_method()
    
    # Unused variable
    unused_var = 10
    
    # Used variable
    x = 5
    print(x)

if __name__ == "__main__":
    main()
