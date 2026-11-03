"""Values computed for a statement go away when the statement is done.

Only the statement is the unit: CPython may drop a value earlier, in the
middle of the statement, so each value here is the last thing a statement
uses."""

import weakref


class Tracked:
    def __init__(self, name):
        self.name = name

    def __del__(self):
        print("del", self.name)

    def __iadd__(self, other):
        return self

    def __bool__(self):
        return True


class Holder:
    pass


def make(name):
    return Tracked(name)


def gone(ref):
    print("weakref gone")


# Module level: a subscript target value, a value used twice, and an
# augmented assignment target object.
d = {}
d["k"] = make("module item")
del d["k"]
print("after module item")

x = y = make("module pair")
del x
del y
print("after module pair")

h = Holder()
h.attr = make("module attr")
make("aug target").attr = 0
print("after aug target")
h.attr += 1
print("after aug")

# A weakref callback fires when the last reference goes.
w = weakref.ref(make("weak"), gone)
print("after weak", w() is None)

# A temporary only borrowed (the operand of `and`) must not linger.
result = make("left") and "right"
print("after and", result)


def function():
    e = {}
    e["k"] = make("function item")
    del e["k"]
    print("after function item")
    value = make("function and") and 1
    print("after function and", value)
    r = weakref.ref(make("function weak"), gone)
    print("after function weak", r() is None)


function()


def loop():
    for i in range(2):
        e = {}
        e[i] = make(f"loop {i}")
        del e[i]
        print("after loop", i)


loop()

# A comprehension's result is dropped once the statement is done with it.
n = len([make(f"comp {i}") for i in range(2)])
print("after comp", n)
n = sum(1 for _ in [make("genexpr")])
print("after genexpr", n)


def augmented():
    holder = Holder()
    holder.attr = make("function attr")
    make("function aug target").attr = 0
    holder.attr += 1
    print("after function aug")


augmented()
print("done")
