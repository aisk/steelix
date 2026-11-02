"""Inheritance from Python-defined and standard library classes."""

import abc
import argparse
import enum
import threading
from typing import NamedTuple, TypedDict


class A:
    def who(self):
        return ["A"]


class B(A):
    def who(self):
        return ["B"] + super().who()


class C(A):
    def who(self):
        return ["C"] + super().who()


class D(B, C):
    def who(self):
        return ["D"] + super().who()


print(D().who(), [k.__name__ for k in D.__mro__])
print(isinstance(D(), A), issubclass(D, C), isinstance(B(), C))


class Color(enum.Enum):
    RED = 1
    GREEN = 2
    BLUE = 3

    def describe(self):
        return f"{self.name.lower()}={self.value}"

    @classmethod
    def default(cls):
        return cls.GREEN

    @property
    def is_warm(self):
        return self is Color.RED


print(Color.RED, Color(2), Color.BLUE.describe(), Color.default(), Color.RED.is_warm)
print(list(Color), Color["BLUE"].value, len(Color))


class Planet(enum.Enum):
    EARTH = (5.97e24, 6.37e6)
    MARS = (6.42e23, 3.39e6)

    def __init__(self, mass, radius):
        self.mass = mass
        self.radius = radius

    @property
    def gravity(self):
        return round(6.67e-11 * self.mass / self.radius**2, 2)


print(Planet.EARTH.gravity, Planet.MARS.gravity)


class Flags(enum.IntFlag):
    READ = 4
    WRITE = 2


print(Flags.READ | Flags.WRITE, int(Flags.READ))


class Shade(enum.Enum):
    def __new__(cls, value, label):
        obj = object.__new__(cls)
        obj._value_ = value
        obj.label = label
        return obj

    DARK = (1, "dark")
    LIGHT = (2, "light")


print(Shade(1).label, Shade.LIGHT.value)


class Point(NamedTuple):
    x: int
    y: int = 0

    def norm1(self):
        return abs(self.x) + abs(self.y)


p = Point(3, -4)
print(p, p.norm1(), Point._fields, Point(1), p._replace(y=1))


class Movie(TypedDict):
    title: str
    year: int


m = Movie(title="Brazil", year=1985)
print(m, Movie.__required_keys__ == {"title", "year"}, Movie.__annotations__)


class Shape(abc.ABC):
    @abc.abstractmethod
    def area(self):
        ...

    def describe(self):
        return f"{type(self).__name__} with area {self.area()}"


class Square(Shape):
    def __init__(self, side):
        self.side = side

    def area(self):
        return self.side**2


try:
    Shape()
except TypeError as err:
    print("TypeError:", err)
print(Square(3).describe(), Shape.__abstractmethods__)


class Worker(threading.Thread):
    def __init__(self, n):
        super().__init__(name=f"worker-{n}")
        self.n = n
        self.result = None

    def run(self):
        self.result = sum(range(self.n))


workers = [Worker(n) for n in (10, 100)] if False else [Worker(10), Worker(100)]
for w in workers:
    w.start()
for w in workers:
    w.join()
print([w.result for w in workers], [w.name for w in workers])


class Upper(argparse.Action):
    def __call__(self, parser, namespace, values, option_string=None):
        setattr(namespace, self.dest, values.upper())


parser = argparse.ArgumentParser(prog="demo")
parser.add_argument("--name", action=Upper)
print(parser.parse_args(["--name", "bob"]))


class Reversed(list):
    def __init__(self, items):
        super().__init__(reversed(items))

    def append(self, item):
        super().insert(0, item)


r = Reversed([1, 2, 3])
r.append(0)
print(r, len(r), isinstance(r, list))


class Default(dict):
    def __missing__(self, key):
        return key * 2


print(Default(a=1)["a"], Default()["xy"])
