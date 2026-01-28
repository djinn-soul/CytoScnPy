from typing import Protocol, runtime_checkable
from abc import abstractmethod

@runtime_checkable
class Renderable(Protocol):
    """A protocol for renderable objects."""
    
    def render(self, context, verbose: bool = False) -> str:
        """Render the object. Parameters should be skipped."""
        ...

class Button:
    def render(self, context, verbose: bool = False) -> str:
        return "Button"

def process(item: Renderable):
    item.render(None)
