"""Locals returned (or yielded) from where they are declared are moved, not
cloned, and display assignments do not build a tuple. Neither changes when
an object goes away (design §8)."""

import weakref


class Tracked:
    def __init__(self, name):
        self.name = name

    def __del__(self):
        print("del", self.name)

    def __repr__(self):
        return f"Tracked({self.name})"


def gone(ref):
    print("weakref gone")


def from_try(name):
    try:
        local = Tracked(name)
        other = Tracked(name + " other")
        print("in try", other.name)
        return local
    except ValueError:
        return None


def from_finally(name):
    try:
        kept = Tracked(name)
        return kept
    finally:
        print("finally runs while", name, "is returned")


def from_handler(name):
    try:
        raise KeyError(name)
    except KeyError as err:
        print("handling", err)
        return err


def from_with(name):
    class Manager:
        def __enter__(self):
            return self

        def __exit__(self, *exc):
            print("exit")
            return False

    with Manager():
        inside = Tracked(name)
        return inside


def from_loop(names):
    for name in names:
        item = Tracked(name)
        if name == "b":
            return item
    return None


def outer_local(name):
    result = Tracked(name)
    try:
        print("try")
        return result
    finally:
        print("result still bound:", result.name)


def swap(a, b):
    a, b = b, a
    print("swapped", a, b)
    first, second = Tracked("first"), Tracked("second")
    print("unpacked", first.name, second.name)
    del first
    print("after del first")
    return a, b


r = from_try("t")
print("got", r)
del r
print("after try")
r = from_finally("f")
print("got", r)
w = weakref.ref(r, gone)
del r
print("after finally", w() is None)
print("got", repr(from_handler("h")))
r = from_with("w")
print("got", r)
del r
r = from_loop(["a", "b", "c"])
print("got", r)
del r
r = outer_local("o")
print("got", r)
del r
print(swap(1, 2))
print(list(x for x in [Tracked("g1"), Tracked("g2")] if x.name))
print("end")
