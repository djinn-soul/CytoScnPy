"""
Complex Scoping Examples
"""

# Global vs Local
x = 10 # Used in f1 via global

def f1():
    global x
    x = 20

# Shadowing
y = 100 # DEAD (overshadowed in f2 and never used globally)

def f2():
    y = 50 # Local y, used
    return y

# Walrus Operator
def f3(items):
    # n defined by walrus and used in condition -> ALIVE
    if (n := len(items)) > 10:
        print(f"Too long: {n}")
    
    # z defined by walrus but unused logic flow?
    # Actually z IS used in the expression itself, but if we don't use z later?
    # Python considers it a local variable.
    if (z := items[0]):
        pass
    # z is now in scope. If not used -> DEAD?
    # But it was used in the `if` check.
    
    # Explicit unused walrus target logic?
    # while (unused := False): pass 

# List Comprehension Leakage (simulated scope checks)
def f4():
    # i in comp is local to comp in Py3
    data = [i for i in range(5)]
    # i should not be available here
    try:
        print(i) 
    except NameError:
        pass
    return data

# Unused variable in comprehension
def f5():
    # x unused in result -> DEAD? No, it's the iteration variable.
    # Usually treated as used.
    # But `_` is idiomatic for unused.
    return [0 for x in range(5)] 

# Class scope quirks
class A:
    val = 1
    # list comp in class scope cannot see class scope vars in Py3 easily 
    # without full qualification in some contexts, but let's test shadowing
    val_list = [val for _ in range(3)] # Access A.val? No, NameError usually?
    # Actually in class body, `val` is accessible to direct expressions, 
    # but scope of list comp is distinct.
    
    def method(self):
        val = 2 # Local shadowing -> DEAD if not used
        return self.val
