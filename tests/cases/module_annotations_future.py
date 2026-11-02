# With `from __future__ import annotations`, module annotations go into one
# `__annotations__` dict, created before the first statement as in CPython.
from __future__ import annotations

print("__annotations__" in dir())
A: int = 1
print(__annotations__)


class Box:
    pass


B: Box = Box()
print(__annotations__)
