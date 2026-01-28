from abc import ABC, abstractmethod

class Processor(ABC):
    @abstractmethod
    def process(self, data) -> str:
        """Abstract method - should NOT be flagged."""
        pass

class ConcreteProcessor(Processor):
    def process(self, data) -> str:
        return f"Processed: {data}"

# Usage
p = ConcreteProcessor()
p.process("test")
