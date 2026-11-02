"""`@dataclass(slots=True)` creates a new class and updates the `__class__`
cell its methods share. It only looks at Python functions and properties, so
when no property uses `super()`, the transpiled methods still refer to the
old class (design §12)."""

# steelix-test: expect-error TypeError: super(type, obj): obj (instance of P) is not an instance or subtype of type (P).
from dataclasses import dataclass


class Base:
    def hello(self):
        return "base"

    @property
    def name(self):
        return "base"


@dataclass(slots=True)
class P(Base):
    x: int

    @property
    def name(self):
        return "p"

    def hello(self):
        return "p+" + super().hello()


print(P(2).x, P(2).name)
print(P(1).hello())
