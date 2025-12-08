x = "global"

def func():
    x = "local"
    print(x) # Should ref func.x

def func2():
    print(x) # Should ref global x

class C:
    x = "class"
    def method(self):
        x = "method"
        print(x) # Should ref method x
        print(self.x) # Should ref class x (via self)
