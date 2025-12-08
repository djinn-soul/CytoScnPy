def outer():
    x = "outer"
    def inner():
        print(x) # Should ref outer.x
    inner()
