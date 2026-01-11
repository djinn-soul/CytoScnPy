
import os
import sys

# Attributes for attr_checks.rs
def attribute_sources(request, req):
    # Flask
    a = request.args
    b = request.form
    c = request.data
    d = request.json
    e = request.cookies
    f = request.files
    g = request.values
    
    # Django
    h = request.GET
    i = request.POST
    j = request.body
    k = request.COOKIES
    
    # Azure
    l = req.params
    m = req.route_params
    n = req.headers
    o = req.form
    
    # Builtin
    p = sys.argv
    q = os.environ
    
    # Chained
    r = request.args.get('key')
    s = request.POST.get('key')

# Intraprocedural Control Flows
async def async_taint_flow(x):
    source = input()
    if x:
        y = source
    else:
        y = "safe"
    
    # Merge
    eval(y)
    
    while x:
        z = source
        eval(z)
        break
    
    for i in range(10):
        w = source
    else:
        w = "safe"
    eval(w)
    
    try:
        t = source
        raise ValueError
    except ValueError:
        t = "caught"
    except:
        t = "other"
    finally:
        eval(t)

def nested_func_scope():
    x = input()
    def inner():
        eval(x) # Closure capture (not fully handled but checks recursion)
        
    async def inner_async():
        eval(x)
        
    inner()

# Call Graph
def a():
    b()
    process_data()

def b():
    c()
    d()

def c():
    d()

def d():
    pass

def process_data():
    pass

class MyClass:
    def method_a(self):
        self.method_b()
        
    def method_b(self):
        pass

# Sinks (fake)
def sink(arg):
    pass

# Direct flows at module level
eval(input())
os.system(sys.argv[0])

# Interprocedural flow (if summary works for return input())
def get_user_data():
    return input()

def flask_endpoint():
    data = get_user_data()
    eval(data)
