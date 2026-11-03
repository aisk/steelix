# Class patterns (design §8): `__match_args__`, keyword attributes,
# dataclasses, enums as value patterns and the builtins that match themselves.

import enum
from dataclasses import dataclass


class Point:
    __match_args__ = ("x", "y")

    def __init__(self, x, y):
        self.x = x
        self.y = y


@dataclass
class Circle:
    center: Point
    radius: float


@dataclass
class Rect:
    corner: Point
    width: float
    height: float


class Color(enum.Enum):
    RED = 1
    GREEN = 2
    BLUE = 3


def where(point):
    match point:
        case Point(0, 0):
            return "origin"
        case Point(0, y):
            return f"on the y axis at {y}"
        case Point(x, 0):
            return f"on the x axis at {x}"
        case Point(x=x, y=y) if x == y:
            return f"diagonal {x}"
        case Point(x, y=y):
            return f"somewhere ({x}, {y})"
        case Point():
            return "a point"
        case _:
            return "not a point"


for p in [Point(0, 0), Point(0, 5), Point(3, 0), Point(2, 2), Point(1, 7), "p"]:
    print(where(p))


def area(shape):
    match shape:
        case Circle(Point(x, y), r) if r > 0:
            return f"circle at {x},{y}: {round(3.14159 * r * r, 2)}"
        case Circle(radius=0):
            return "a dot"
        case Rect(Point(x=x0, y=y0), w, h):
            return f"rect from {x0},{y0}: {w * h}"
        case Rect(width=0) | Rect(height=0) | Circle(radius=-1):
            return "flat"
        case Point(x=Point() | int()):
            return "point with a point or int x"
    return "unknown"


shapes = [Circle(Point(1, 2), 2), Circle(Point(0, 0), 0), Rect(Point(1, 1), 2, 3)]
shapes += [Rect(Point(1, 1), 0, 3), Point(4, 4), Point(Point(0, 0), 1), Point("x", 1)]
for shape in shapes:
    print(area(shape))


def paint(color):
    match color:
        case Color.RED:
            return "warm"
        case Color.GREEN | Color.BLUE:
            return "cool"
        case Color():
            return "another color"
    return "not a color"


for color in [Color.RED, Color.BLUE, 1, "red"]:
    print(paint(color))


def kind(value):
    """`bool(x)`, `int(x)`, `str(x)`, ...: a single positional subpattern
    matches the subject itself."""
    match value:
        case bool(b):
            return f"bool {b}"
        case int(0) | float(0.0):
            return "zero"
        case int(n) if n < 0:
            return f"negative int {n}"
        case int(n):
            return f"int {n}"
        case float(f):
            return f"float {f}"
        case str("") | bytes(b""):
            return "empty text"
        case str(s):
            return f"str {s!r}"
        case bytearray(b):
            return f"bytearray {bytes(b)!r}"
        case list([]) | tuple(()):
            return "empty sequence"
        case list([first, *rest]):
            return f"list starting with {first}, {len(rest)} more"
        case tuple(t):
            return f"tuple of {len(t)}"
        case dict({"k": v}):
            return f"dict with k={v}"
        case set() | frozenset():
            return "a set"
    return "something else"


values = [True, 0, 0.0, -4, 7, 2.5, "", b"", "hi", bytearray(b"ab"), [1, 2, 3], (1, 2), []]
values += [(), {"k": 9}, {1}, frozenset(), None]
for value in values:
    print(repr(value), "->", kind(value))


class Sub(int):
    """A subclass of a builtin without `__match_args__` matches itself too."""


class Named(int):
    __match_args__ = ("real", "imag")


match Sub(5):
    case int(n):
        print("Sub matched as int", n, type(n).__name__)

match Named(5):
    case Named(real, imag):
        print("Named by __match_args__", real, imag)


class Missing:
    __match_args__ = ("a", "b")

    def __init__(self):
        self.a = 1


match Missing():
    case Missing(a, b):
        print("both")
    case Missing(a):
        print("only a", a)

match Missing():
    case Missing(c=1):
        print("c")
    case Missing(a=1):
        print("a is 1")
