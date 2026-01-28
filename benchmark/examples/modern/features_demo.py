import enum
from typing import NamedTuple

# 1. Mixin Penalty (-60% confidence -> should be ignored if threshold > 40)
class AccessMixin:
    def check_access(self):
        print("Access checked")

# 2. Base Class Penalty (-50% confidence)
class NotificationBase:
    def send(self):
        raise NotImplementedError

# 3. Enum members (Should now be marked as UNUSED because we removed the implicit usage)
# User specifically requested that unused Enum members be flagged.
class Color(enum.Enum):
    RED = 1
    GREEN = 2

# 4. Optional Dependencies (Should be marked as used)
try:
    import pandas as pd
    import numpy as np
except ImportError:
    pass

# 5. Lifecycle methods (-30% / -40% confidence)
class Widget:
    def on_click(self):
        pass

    def watch_value(self):
        pass

    def compose(self):
        pass
