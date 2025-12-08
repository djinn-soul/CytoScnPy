import math

def calculate_stuff(x, y, z):
    a = x + y * z
    b = math.sqrt(a) / (x - y)
    c = a ** 2 + b ** 3
    d = "result: " + str(c)
    return d

data = [1, 2, 3, 4, 5]
mapped = list(map(lambda x: x * 2, data))
filtered = [x for x in mapped if x > 5]

dictionary = {"a": 1, "b": 2, "c": 3}
keys = dictionary.keys()
values = dictionary.values()
