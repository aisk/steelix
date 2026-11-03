"""The entry script of the attribute index tests."""

import other
import shapes
from shapes import Point as P
from shapes import Size


def scale(p: P, k):
    p.x *= k
    p.y = p.y * k


def origin():
    o = P(0, 0)
    o.label = "origin"
    return o


def maybe(flag):
    q = P(1, 2)
    if flag:
        q = None
    return q.x


def closure():
    r = P(1, 2)

    def rebind():
        nonlocal r
        r = None

    rebind()
    return r.y


def show(thing, name: str):
    print(thing.x, getattr(thing, name), thing.width)


def configure(p: "P | None", **values):
    print(getattr(p, "y"), vars(p))
    for key, value in values.items():
        setattr(p, key, value)


def reset():
    shapes.Point.count = 0
    del shapes.Odd.LIMIT


s = Size(1, 2)
print(scale(origin(), 2), maybe(False), closure(), other.far(P(3, 4)))
