"""Calls of transpiled functions through every CPython call path (vectorcall,
`tp_call`, bound methods, `functools.partial`), with and without the fast
paths for plain positional calls."""

import functools
import threading


def show(function, /, *args, **kwargs):
    try:
        print(function(*args, **kwargs))
    except TypeError as e:
        print("TypeError:", e)


def pair(a, b):
    return (a, b)


def defaults(a, b=2, c=3):
    return (a, b, c)


def mixed(a, /, b, *args, c, d=4, **kwargs):
    return (a, b, args, c, d, kwargs)


def posonly_kwargs(a, /, **kwargs):
    return (a, kwargs)


def star(*args):
    return args


# Plain positional calls, which need no binding.
show(pair, 1, 2)
show(pair, 1)
show(pair, 1, 2, 3)
show(pair, b=2, a=1)
show(pair, 1, a=2)

# Missing arguments filled from `__defaults__`, read at call time.
show(defaults, 1)
show(defaults, 1, 5)
show(defaults, 1, 5, 6)
defaults.__defaults__ = (7,)
show(defaults, 1)
show(defaults, 1, 2)
defaults.__defaults__ = (8, 9, 10, 11)
show(defaults, 1)
defaults.__defaults__ = None
show(defaults, 1, 2)
defaults.__defaults__ = (2, 3)

# Keywords, `*args` and `**kwargs`.
show(mixed, 1, 2, c=3)
show(mixed, 1, 2, 3, 4, c=5, d=6, e=7, f=8)
show(mixed, 1, b=2, c=3, z=0)
show(mixed, 1, 2)
show(mixed, a=1, b=2, c=3)
show(posonly_kwargs, 1, a=2)
show(lambda *a, **k: (a, k), 1, x=2)
t = (1, 2)
print(star(*t) == t, star(*t) is t, star() == ())
show(pair, **{"a": 1, "b": 2})
try:
    pair(1, **{2: 3})
except TypeError as e:
    print("TypeError:", e)

# `tp_call`: `f.__call__` and `type(f).__call__` bind the same way.
show(pair.__call__, 1, 2)
show(pair.__call__, 1)
show(type(pair).__call__, pair, 1, b=2)
show(defaults.__call__, 1, c=5)
show(mixed.__call__, 1, 2, c=3, e=4)
show(mixed.__call__, 1, 2, b=3, c=4)


class Box:
    def __init__(self, value, extra=0):
        self.value = value + extra

    def get(self):
        return self.value

    def add(self, n, times=1):
        return self.value + n * times

    @classmethod
    def make(cls, value):
        return cls(value)

    @staticmethod
    def twice(x):
        return 2 * x


class Child(Box):
    def get(self):
        return super().get() + 100


box = Box(1)
print(box.get(), box.add(2), box.add(2, 3), box.add(n=2, times=4))
print(Box.get(box), Box.add(box, 5), Box(1, extra=2).get())
print(type(box.get).__name__, box.get.__self__ is box, box.get.__func__ is Box.__dict__["get"])
print(box.get == box.get, box.get is box.get, Box.get is Box.__dict__["get"])
print(Box.make(5).get(), box.make(6).get(), Box.twice(4), box.twice(5))
print(Child(1).get(), Child.make(2).get())
show(box.add)
show(box.add, 1, 2, 3)
show(Box.get)
getter = box.get
box.value = 42
print(getter(), getattr(box, "add")(1))
# A function stored on the instance is not bound.
box.pair = pair
print(box.pair(1, 2))

# `functools.partial` and `partialmethod`.
print(functools.partial(pair, 1)(2), functools.partial(defaults, c=0)(1))
print(functools.partial(Box.add, box)(1), functools.partial(box.add, times=2)(1))


class WithPartial:
    value = 10
    add = functools.partialmethod(Box.add, times=3)


print(WithPartial().add(1))


# Recursion through Python calls ends in `RecursionError`.
def through(n, f):
    return f(n + 1, f)


try:
    through(0, through)
except RecursionError:
    print("RecursionError")


# An exception caught in a function called from a new thread is released
# right away, like in CPython.
class Noisy:
    def __del__(self):
        print("deleted")


def catch():
    try:
        raise ValueError(Noisy())
    except ValueError:
        print("caught")
    print("after")


worker = threading.Thread(target=catch)
worker.start()
worker.join()
