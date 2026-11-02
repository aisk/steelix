"""Statements in a class body run in the class namespace."""

import sys
import traceback


def show_tb(err):
    return [(f.name, f.lineno) for f in traceback.extract_tb(err.__traceback__)]


class Body:
    import os.path as paths
    from math import sqrt as root

    total = 0
    for i in range(4):
        total += i
    while total > 10:
        total -= 1
    names = []
    if sys.version_info >= (3, 0):

        def version(self):
            return "py3"

    else:

        def version(self):
            return "py2"

    try:
        missing = {}["key"]
    except KeyError as err:
        caught = repr(err)
    finally:
        cleanup = True
    with open(__file__) as handle:
        first = handle.readline().strip()
    temp = 1
    del temp
    try:
        del temp
    except NameError as err:
        name_error = str(err)
    x = 1; squares = [x * x for x in range(3)]; y = 2
    label = (lambda self: "lambda " + type(self).__name__)
    shadow = len
    len = 5
    used = shadow([1, 2])
    builtin = print.__name__
    assert total == 6, "total"


b = Body()
print(Body.total, Body.i, Body.version(b), Body.caught, Body.cleanup, Body.first)
print(Body.name_error, Body.squares, Body.x, Body.y, b.label(), Body.used, Body.len)
print(Body.paths.join("a", "b"), Body.root(16), Body.builtin, hasattr(Body, "err"))
print(Body.label.__qualname__, Body.__static_attributes__)


class Generic:
    def __class_getitem__(cls, item):
        return f"{cls.__name__}[{item.__name__}]"

    def __init_subclass__(cls, **kwargs):
        cls.created = True


class Sub(Generic):
    pass


print(Generic[int], Sub.created, type(Generic.__dict__["__class_getitem__"]).__name__)


class New:
    def __new__(cls, *args):
        print("new", cls.__name__, args)
        return super().__new__(cls)

    def __init__(self, value):
        self.value = value


print(New(3).value, type(New.__dict__["__new__"]).__name__)


def failing_body():
    class Failing:
        value = 1 / 0


try:

    class Failing:
        before = 1
        value = 1 / 0

except ZeroDivisionError as err:
    print("ZeroDivisionError", show_tb(err))

try:

    class FailingKept:
        values = [1 / 0 for _ in "x"]

except ZeroDivisionError as err:
    print("ZeroDivisionError", show_tb(err))


class Annotated:
    alias = int

    def method(self, x: alias) -> "Annotated":
        return x

    def kept(self, x: alias) -> list[alias]:
        return [x for _ in "x"]


print(Annotated.method.__annotations__, Annotated.kept.__annotations__)


class Uses:
    def __getattr__(self, name):
        return name.upper()

    def __setattr__(self, name, value):
        super().__setattr__(name, value * 2)

    def __contains__(self, item):
        return item == "x"

    def __lt__(self, other):
        return True

    def __enter__(self):
        return self

    def __exit__(self, *exc):
        print("exit", exc[0])
        return True


u = Uses()
u.v = 2
print(u.v, u.missing, "x" in u, u < u, sorted([u, u])[0] is u)
with u as same:
    raise ValueError("suppressed")
print(same is u)
