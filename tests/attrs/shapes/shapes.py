"""Classes for the attribute index tests (`tests/test_attrs.py`)."""

import functools
from dataclasses import dataclass


class Point:
    UNITS = ("m", "km")
    count = 0

    def __init__(self, x, y):
        self.x = x
        self.y = y
        self.__secret = x * y
        Point.count += 1

    def norm(self):
        return (self.x**2 + self.y**2) ** 0.5

    def move(self, dx):
        self.x += dx
        return self

    def later(self):
        def inner():
            return self.y

        return inner, lambda: self.__secret


class Point3(Point):
    def __init__(self, x, y, z):
        super().__init__(x, y)
        self.z = z

    def flat(self):
        return Point(self.x, self.y)


@dataclass
class Size:
    width: int
    height: int = 0


class Meta(type):
    pass


class Base:
    pass


class Odd(Base, metaclass=Meta):
    __slots__ = ("a", "_b")
    LIMIT = 10
    if LIMIT > 5:
        big = True

    def __init__(self, a, /, b=1, *rest, key, flag=False, **extra):
        self.a = a
        self._b = b

    @property
    def b(self):
        return self._b

    @b.setter
    def b(self, value):
        self._b = value

    @staticmethod
    def make(x):
        return Odd(x, key=1)

    @classmethod
    def build(cls, *args):
        cls.LIMIT = 20
        return cls(*args, key=2)

    @functools.cache
    def cached(self):
        return vars(self)

    def __hash__(self):
        return 1

    def __getattr__(self, name):
        return getattr(self.__dict__, name)
