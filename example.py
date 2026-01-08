import msgspec


class Point(msgspec.Struct):
    """A 2D Point structure."""

    x: int
    y: int


data = msgspec.json.decode(b"{x:1,y:5}", type=Point)
# print(data)
