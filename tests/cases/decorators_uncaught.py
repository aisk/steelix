"""Decorators of methods and functions: evaluation order, names, qualified
names, and the traceback of a decorator that raises."""

import functools


def show(label):
    print("evaluate", label)

    def decorate(fn):
        print("apply", label, "to", fn.__qualname__)
        return fn

    return decorate


def default(label):
    print("default", label)
    return label


def boom(fn):
    raise ValueError("boom applied to " + fn.__qualname__)


class Tags:
    def __init__(self):
        self.seen = []

    def tag(self, fn):
        self.seen.append(fn.__qualname__)
        return fn


tags = Tags()


@show("first")
@show("second")
@show("third")
def ordered(a=default("a"), *, b=default("b")):
    return a + b


print(ordered(), ordered.__qualname__)


class Shape:
    @show("method")
    def area(self, scale=default("scale")):
        return 2 * scale

    @property
    def label(self):
        return self._label

    @label.setter
    def label(self, value):
        self._label = value.upper()

    @label.deleter
    def label(self):
        self._label = "deleted"

    @classmethod
    def unit(cls):
        return cls.__name__

    @staticmethod
    def double(x):
        return 2 * x

    @tags.tag
    def tagged(self):
        return "tagged"

    @functools.lru_cache(maxsize=None)
    def cached(self, n):
        return n * 10

    def type(self):
        return "type is a Rust keyword"

    def match(self):
        return "so is match"

    class Inner:
        def method(self):
            return "inner"

        class Deepest:
            @staticmethod
            def method():
                return "deepest"


s = Shape()
s.label = "box"
print(s.label, Shape.unit(), Shape.double(4), s.area(), s.tagged(), s.cached(4))
del s.label
print(s.label, s.type(), s.match(), Shape.Inner().method(), Shape.Inner.Deepest.method())
print(tags.seen)
for fn in [
    ordered,
    Shape.area,
    Shape.label.fget,
    Shape.label.fset,
    Shape.label.fdel,
    Shape.unit,
    Shape.double,
    Shape.tagged,
    Shape.cached.__wrapped__,
    Shape.type,
    Shape.match,
    Shape.Inner.method,
    Shape.Inner.Deepest.method,
]:
    print(fn.__name__, fn.__qualname__)


def factory(prefix):
    local = show("local " + prefix)

    @local
    @functools.wraps(factory)
    def made(x=default(prefix)):
        return prefix + x

    @local
    def plain(y=local):
        return y.__qualname__

    return made, plain


made, plain = factory("p")
print(made(), made.__name__, made.__qualname__, made.__wrapped__.__qualname__)
print(plain(), plain.__name__, plain.__qualname__)


@(chosen := show("walrus"))
def walrus():
    return "walrus"


print(walrus(), chosen.__qualname__)

try:

    @show("never applied")
    @boom
    def failing():
        pass

except ValueError as e:
    print("caught", e)

print("failing" in globals())


try:

    class Broken:
        @show("class body")
        @boom
        def method(self):
            pass

except ValueError as e:
    print("caught", e)


def build():
    @show("outer")
    @boom
    @show("inner")
    def method(x=default("x")):
        return x

    return method


print("building")
build()
