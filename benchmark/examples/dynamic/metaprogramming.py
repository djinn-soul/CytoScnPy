"""
Metaprogramming Example
Run with: cytoscnpy examples/metaprogramming.py
"""

# Decorator
def my_decorator(func):
    def wrapper():
        print("Something is happening before the function is called.")
        func()
        print("Something is happening after the function is called.")
    return wrapper

@my_decorator
def say_whee():
    print("Whee!")

# Metaclass
class Meta(type):
    def __new__(cls, name, bases, dct):
        x = super().__new__(cls, name, bases, dct)
        x.attr = 100
        return x

class MyClass(metaclass=Meta):
    pass

def main():
    say_whee()
    print(MyClass.attr)

if __name__ == "__main__":
    main()
