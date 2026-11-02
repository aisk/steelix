"""Functions defined in module level compound statements: nested fn items of
`exec_module`, bound in the globals when the `def` runs."""

import sys

counter = 10

if not hasattr(sys, "no_such_attribute"):
    def helper(x, scale=2):
        """Doc of helper."""
        return x * scale + counter

    def recurse(n):
        return 0 if n == 0 else 1 + recurse(n - 1)
else:
    def helper(x, scale=2):
        return -1

for i in range(2):
    def in_loop(y=i):
        return y

    print(in_loop(), helper(i), recurse(3))

try:
    def in_try():
        return helper.__qualname__, helper.__doc__, helper.__module__
finally:
    print(in_try())


def top():
    return helper(1, scale=3)


print(top())


def f():
    return "top"


if counter > 5:
    # Rebinding a module function: calls go through the globals.
    def f():
        return "conditional"


def g():
    return f()


print(f(), g(), f.__qualname__)
