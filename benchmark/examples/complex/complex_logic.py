def complex_function(a, b, c):
    """A function with high complexity."""
    if a > 0:
        if b > 0:
            if c > 0:
                return a + b + c
            else:
                return a + b - c
        elif b < 0:
            return a - b
        else:
            return a
    elif a < 0:
        while b > 0:
            b -= 1
            if b == 5:
                break
            elif b == 3:
                continue
        return b
    else:
        for i in range(10):
            if i % 2 == 0 and c > 0:
                print(i)
            elif i % 3 == 0 or c < 0:
                print(i * 2)
    return 0

class ComplexClass:
    def method_a(self, x):
        if x:
            return True
        return False

    def method_b(self, y):
        return [i for i in range(y) if i % 2 == 0 if i > 5]

    def recursive(self, n):
        if n <= 1:
            return 1
        return n * self.recursive(n - 1)
