
import math
from os import path as p, sep
import sys

# Globals and Nonlocals
G = 1
def scope_test():
    global G
    nonlocal N
    x = 1

# Async
async def async_func():
    async for x in range(10):
        async with open('file') as f:
            await f.read()

# Class and Methods
class MyClass:
    def method(self):
        pass

# Control Flow
def control_flow(x):
    if x:
        pass
    elif x > 1:
        pass
    else:
        pass
    
    while x:
        break
        continue
    else:
        pass
    
    for i in range(10):
        pass
    else:
        pass
        
    try:
        raise ValueError("error") from None
    except ValueError as e:
        pass
    except (TypeError, IndexError):
        pass
    else:
        pass
    finally:
        pass
    
    with open('x') as f, open('y'):
        pass
    
    match x:
        case 1: pass
        case _: pass

# Operators
def operators():
    a = 1 + 2 - 3 * 4 / 5 // 6 % 7 @ 8 ** 9
    b = 1 << 2 >> 3 | 4 ^ 5 & 6
    c = ~1
    d = not True
    e = +1
    f = -1
    
    # AugAssign
    x = 1
    x += 1; x -= 1; x *= 1; x /= 1; x //= 1; x %= 1; x @= 1; x **= 1
    x <<= 1; x >>= 1; x |= 1; x ^= 1; x &= 1
    
    # Comparison
    res = (1 == 1) and (1 != 2) or (1 < 2) and (1 <= 2) and (1 > 0) and (1 >= 0)
    res = (1 is 1) and (1 is not 2) and (1 in [1]) and (1 not in [2])
    
    # Subscript / Slice
    l = [1, 2, 3]
    v = l[0]
    s = l[0:1:2]
    
    # Expressions
    val = (x := 10)
    lam = lambda x: x + 1
    ternary = 1 if True else 0
    
    # Collections
    lst = [1, 2, *l]
    tup = (1, 2)
    dct = {'a': 1, **{'b': 2}}
    st = {1, 2}
    
    # Comprehensions
    lc = [x for x in range(10) if x > 5]
    sc = {x for x in range(10)}
    dc = {x: x for x in range(10)}
    gen = (x for x in range(10))
    
    # Yield
    yield 1
    yield from gen
    
    # F-string
    s = f"val: {val!r:.2f}"
    b = b"bytes"
    
    # Delete
    del x
    
    # Assert
    assert True, "msg"

# Type Alias (Python 3.12)
type MyInt = int
