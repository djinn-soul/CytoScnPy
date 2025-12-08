def calculate(expression):
    # 'x' and 'y' are defined but not statically used
    x = 10
    y = 20
    # eval makes the module dynamic, so all locals should be considered used
    return eval(expression)

if __name__ == "__main__":
    calculate("x + y")
