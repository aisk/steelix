"""The global statement, module globals used from functions, pass and assert."""

import sys

counter = 0
names = []


def bump(step):
    global counter
    counter = counter + step
    counter += 1
    return counter


def remember(name):
    names.append(name)
    return len(names)


def reset():
    global counter, names
    counter = 0
    names = []
    del counter


def check(x):
    assert x > 0
    assert x < 100, "too big"
    assert isinstance(x, int), f"not an int: {x!r}"
    pass
    return x


def define():
    global late
    late = "defined late"


print(bump(2), bump(3), counter)
print(remember("a"), remember("b"), names)
reset()
print(hasattr(sys.modules[__name__], "counter"), names)
define()
print(late)
print(check(5))
check(500)
