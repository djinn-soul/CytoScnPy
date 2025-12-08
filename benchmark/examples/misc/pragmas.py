"""
Pragma Suppression Example
Run with: cytoscnpy examples/pragmas.py
"""

def unused_but_ignored():  # pragma: no cytoscnpy
    print("I am unused but ignored")

def unused_and_reported():
    print("I am unused and will be reported")

def main():
    # This variable is unused but ignored
    x = 10  # pragma: no cytoscnpy
    
    # This variable is unused and reported
    y = 20

if __name__ == "__main__":
    main()
