from typing import TypeAlias, NewType
from typing_extensions import TypeAliasType

# PEP 695 / TypeAliasType
Vector = TypeAliasType("Vector", list[float])

# NewType
UserId = NewType("UserId", int)

# TypeAlias annotation
JsonValue: TypeAlias = dict[str, "JsonValue"] | list["JsonValue"] | str | int | float | bool | None

def process_data(v: Vector, uid: UserId, data: JsonValue):
    print(v, uid, data)
