def method_misuse_primitives():
    s = "hello"
    s.append("world")  # CSP-D301: str has no append

    l = [1, 2, 3]
    l.strip()  # CSP-D301: list has no strip

    i = 10
    i.startswith("1")  # CSP-D301: int has no startswith

    d = {"a": 1}
    d.add(2)  # CSP-D301: dict has no add

    st = {1, 2}
    st.append(3)  # CSP-D301: set has no append

def method_misuse_scope():
    x = "outer"
    
    def inner():
        x = [1, 2] # Shadowing
        x.append(3) # Safe: x is list here
    
    x.append("fail") # CSP-D301: x is str in this scope

def method_misuse_reassign():
    x = "string"
    x.append("fail") # CSP-D301
    
    x = [1, 2]
    x.append(3) # Safe: x is now list

def method_misuse_ann_assign():
    x: str = "annotated"
    x.append("fail") # CSP-D301

def correct_usage():
    s = "hello"
    s.upper()
    
    l = []
    l.append(1)
    
    d = {}
    d.get("key")
