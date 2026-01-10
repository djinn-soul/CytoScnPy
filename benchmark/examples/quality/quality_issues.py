"""
Code Quality Issues Example
Run with: cytoscnpy examples/quality_issues.py --quality
"""

# CSP-L001: Mutable default argument
def append_to(element, to=[]):
    to.append(element)
    return to

# CSP-C303: Too many arguments (default max is 5)
def too_many_args(a, b, c, d, e, f):
    return a + b + c + d + e + f

# CSP-Q302: Deep nesting (default max depth is 3)
def deep_nesting():
    if True:
        if True:
            if True:
                if True:
                    print("Too deep!")

# CSP-L002: Bare except
def bare_except():
    try:
        1 / 0
    except:
        print("Caught something")

# CSP-L003: Dangerous comparison
def dangerous_comparison(x):
    if x == True:
        print("Use 'is True'")
    if x == None:
        print("Use 'is None'")

# CSP-Q301: High Cyclomatic Complexity
def complex_function(x):
    if x == 1:
        return 1
    elif x == 2:
        return 2
    elif x == 3:
        return 3
    elif x == 4:
        return 4
    elif x == 5:
        return 5
    elif x == 6:
        return 6
    elif x == 7:
        return 7
    elif x == 8:
        return 8
    elif x == 9:
        return 9
    elif x == 10:
        return 10
    else:
        return 0
