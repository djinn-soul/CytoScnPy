async def async_func(a, b):
    async for x in a:
        if x > b:
            await x.process()

def match_example(status):
    match status:
        case 400:
            return "Bad request"
        case 404:
            return "Not found"
        case 418:
            return "I'm a teapot"
        case _:
            return "Something's wrong with the internet"

def walrus_example(data):
    if (n := len(data)) > 10:
        print(f"List is too long ({n} elements)")

def pos_only(a, b, /, c, d, *, e, f):
    print(a, b, c, d, e, f)
