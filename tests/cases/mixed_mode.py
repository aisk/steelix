"""Mixed mode: what steelix can not lower yet stays Python and runs in place."""

import inspect
import os

events = []


def log(event):
    events.append(event)
    return event


log("native statement before the class")


class Counter:
    """A native class whose methods call native functions."""

    instances = log("class body")

    def __init__(self, start):
        self.value = start

    def bump(self):
        # A method calling a native function.
        self.value = double(self.value)
        return self


def double(x):
    return x * 2


def evens(limit):
    type Number = int
    for n in range(limit):
        if n % 2 == 0:
            yield n


def use_counter(start):
    # Native code calling the class and the Python generator through globals.
    counter = Counter(start).bump().bump()
    return counter.value + sum(evens(5))


log("native statement after the class")
squares = list(s for s in map(double, range(4)) for _ in "1")
print(squares, use_counter(3))
print(Counter.__module__, Counter.bump.__qualname__, evens.__qualname__, evens.__module__)
print(os.path.basename(evens.__code__.co_filename), evens.__code__.co_firstlineno)
print(inspect.getsource(evens).splitlines()[0])

try:
    import json
except ImportError:
    json = None
print(json.dumps({"kept": True}))

FLAG = True
if FLAG:
    def pick():
        return "first"
else:
    def pick():
        return "second"
print(pick())


def twice():
    return "native"


def twice():
    return "redefined, kept as Python"


print(twice())


def set_mode(mode):
    global MODE
    MODE = mode


set_mode("set by native code")
MODES = {**{"a": MODE}, "b": MODE}
print(MODES)


def trace(fn):
    log(f"decorating {fn.__name__}")
    return fn


@trace
def chunks(items):
    yield from items


print(list(chunks([1, 2])))
print(events)

# `:=` in a comprehension binds a global, here shadowing a builtin.
[(min := v) for v in (3, 7)]
print(min)
