"""
Framework Patterns Example
Run with: cytoscnpy examples/frameworks.py
"""

# Flask-like route
def route(path):
    def decorator(func):
        return func
    return decorator

@route("/index")
def index():
    return "Hello World"

# Django-like model
class Model:
    pass

class User(Model):
    # These fields might look unused but are used by the framework
    name = "String"
    email = "String"

    def __str__(self):
        return self.name

# FastAPI-like dependency
def Depends(dependency):
    pass

def get_db():
    return "db"

def read_users(db = Depends(get_db)):
    return db

def main():
    # Simulate framework entry points
    print(index())
    u = User()
    print(u)
    print(read_users())

if __name__ == "__main__":
    main()
