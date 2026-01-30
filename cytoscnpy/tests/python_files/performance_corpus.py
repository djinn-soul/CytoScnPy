
# CSP-P001: Membership in List Literal
def bad_membership(x):
    if x in [1, 2, 3]:  # Bad
        pass
    if x in list((1, 2, 3)): # Bad? No, strictly literal list check
        pass
    if x in [y for y in range(10)]: # Bad (list comp)
        pass

def good_membership(x):
    if x in {1, 2, 3}:  # Good (set)
        pass
    if x in (1, 2, 3):  # Tuple (better than list)
        pass

# CSP-P002: Readlines
def bad_readlines():
    with open("foo.txt") as f:
        for line in f.readlines(): # Bad
            pass

def good_readlines():
    with open("foo.txt") as f:
        for line in f: # Good
            pass

# CSP-P003: String Concat in Loop
def bad_concat(items):
    s = ""
    for item in items:
        s += item # Bad (CSP-P003)

    i = 0
    while i < 10:
        s += str(i) # Bad
        i += 1

def good_concat(items):
    s = "".join(items) # Good

# CSP-P004: Useless Cast
def bad_casts():
    for x in list(range(100)): # Bad
        pass
    for x in list(map(str, range(100))): # Bad
        pass
    for x in list(filter(None, range(100))): # Bad
        pass
    for x in tuple(range(100)): # Bad
        pass

def good_casts():
    for x in range(100):
        pass

# CSP-P005: Regex/AST in Loop
import re
import ast

def bad_regex_loop(lines):
    for line in lines:
        re.compile("foo") # Bad
        re.search("bar", line) # Bad
        re.match("baz", line) # Bad
        ast.parse("x=1") # Bad

def good_regex_loop(lines):
    pattern = re.compile("foo")
    for line in lines:
        pattern.search(line) # Good

def bad_nested_regex_loop():
    while True:
        re.compile("foo") # Bad
