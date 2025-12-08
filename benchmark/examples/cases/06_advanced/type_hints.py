from typing import TYPE_CHECKING

if TYPE_CHECKING:
    # Used only in type hint string -> ALIVE
    from os import path
    # Unused in type checking block -> DEAD
    import sys

def process_path(p: "path.PathLike"):
    pass

class MyNode:
    # Forward reference in annotation -> ALIVE
    def set_parent(self, parent: "MyNode"):
        pass
    
    # Forward reference in string literal -> ALIVE
    children: "list[MyNode]" = []
    
    def method(self):
        # Local import used in annotation -> ALIVE (if tool supports it)
        # or at least shouldn't crash
        from datetime import datetime
        x: datetime = datetime.now()
        return x

def unused_type_import():
    # Imported inside function but only for type hint? 
    # Runtime import unused -> DEAD
    import re
    x: "re.Pattern" = None
