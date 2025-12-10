"""
Metaclass Pattern Examples for Benchmarking
Tests various metaclass usage patterns to verify the fix for Bug #5.

EXPECTED TRUE POSITIVES (should be flagged as unused):
- UnusedMeta: Never used as metaclass
- AbstractStyleClass: Defined but not instantiated
- ValidatedModel: Defined but not instantiated  
- ValidatedModel.validate_name: Method never called
- MultiKeywordClass.name: Attribute never read

KNOWN FALSE POSITIVES (tool limitations - should NOT be flagged):
- ConcretePlugin: Registered via PluginRegistry metaclass side-effect
- MultiKeywordClass: Registered via PluginRegistry metaclass side-effect
- args param in SingletonMeta.__call__: Used via super().__call__(*args, **kwargs)

CORRECTLY NOT FLAGGED (metaclass fix working):
- Meta, ABCStyleMeta, SingletonMeta, ValidatorMeta, PluginRegistry
"""

from abc import ABCMeta


# =============================================================================
# Pattern 1: Basic Custom Metaclass
# =============================================================================
class Meta(type):
    """Custom metaclass that adds an attribute."""
    def __new__(cls, name, bases, dct):
        x = super().__new__(cls, name, bases, dct)
        x.attr = 100
        return x


class MyClass(metaclass=Meta):
    """Class using custom metaclass."""
    pass


# =============================================================================
# Pattern 2: ABC-style Metaclass
# =============================================================================
class ABCStyleMeta(ABCMeta):
    """Metaclass extending ABCMeta."""
    def __new__(mcs, name, bases, namespace):
        cls = super().__new__(mcs, name, bases, namespace)
        cls._registry = []
        return cls


class AbstractStyleClass(metaclass=ABCStyleMeta):
    """Class using ABC-style metaclass."""
    pass


# =============================================================================
# Pattern 3: Singleton Metaclass Pattern
# =============================================================================
class SingletonMeta(type):
    """Metaclass implementing singleton pattern."""
    _instances = {}

    def __call__(cls, *args, **kwargs):
        if cls not in cls._instances:
            cls._instances[cls] = super().__call__(*args, **kwargs)
        return cls._instances[cls]


class Singleton(metaclass=SingletonMeta):
    """Singleton class."""
    def __init__(self):
        self.value = 42


# =============================================================================
# Pattern 4: Validation Metaclass
# =============================================================================
class ValidatorMeta(type):
    """Metaclass that validates class attributes."""
    def __new__(mcs, name, bases, namespace):
        for key, value in namespace.items():
            if key.startswith('validate_'):
                if not callable(value):
                    raise TypeError(f"{key} must be callable")
        return super().__new__(mcs, name, bases, namespace)


class ValidatedModel(metaclass=ValidatorMeta):
    """Model with validation."""
    def validate_name(self, name):
        return isinstance(name, str)


# =============================================================================
# Pattern 5: Plugin Registry Metaclass
# =============================================================================
class PluginRegistry(type):
    """Metaclass that registers all subclasses."""
    plugins = {}
    
    def __new__(mcs, name, bases, namespace):
        cls = super().__new__(mcs, name, bases, namespace)
        if bases:  # Don't register the base class itself
            mcs.plugins[name] = cls
        return cls


class BasePlugin(metaclass=PluginRegistry):
    """Base plugin class."""
    pass


class ConcretePlugin(BasePlugin):
    """A concrete plugin implementation."""
    pass


# =============================================================================
# Pattern 6: Unused Metaclass (should be detected as unused)
# =============================================================================
class UnusedMeta(type):
    """This metaclass is never used - should be flagged as unused."""
    pass


# =============================================================================
# Pattern 7: Multiple Keywords with Metaclass
# =============================================================================
class MultiKeywordClass(BasePlugin, metaclass=PluginRegistry):
    """Class with both base class and explicit metaclass."""
    name = "multi"


def main():
    """Entry point for testing."""
    # Test basic metaclass
    print(f"MyClass.attr = {MyClass.attr}")
    
    # Test singleton
    s1 = Singleton()
    s2 = Singleton()
    assert s1 is s2, "Singleton failed"
    
    # Test plugin registry
    print(f"Registered plugins: {list(PluginRegistry.plugins.keys())}")


if __name__ == "__main__":
    main()
