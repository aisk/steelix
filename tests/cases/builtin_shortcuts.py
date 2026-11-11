"""Builtins called through `rt::builtins`, and the calls that are not."""

import builtins


def shortcuts(xs):
    print(list(), dict(), set(), tuple(), int(), float())
    print(sorted(xs), max(xs), min(xs), sum(xs), list(reversed(xs)))
    print(list(enumerate(xs)), list(zip(xs, xs)), list(map(str, xs)))
    print(any(xs), all(xs), hash(1), ord("a"), chr(98), divmod(7, 2), round(2.5))
    print(int("12"), int("ff", 16), float("1.5"), max(1, 2, 3), next(iter(xs), None))
    print(hasattr(xs, "append"), getattr(xs, "missing", "default"), callable(len))


def not_shortcuts(xs):
    # Keyword and star arguments keep the general call.
    print(sorted(xs, reverse=True), max(*xs), int("7", base=8), max([], default=0))


def errors():
    try:
        int("x")
    except ValueError as e:
        print(type(e).__name__, e)
    try:
        max()
    except TypeError as e:
        print(type(e).__name__, e)
    try:
        next(iter([]))
    except StopIteration:
        print("stop")


def sum(xs):
    return "module level sum"


shortcuts([3, 1, 2])
not_shortcuts([3, 1, 2])
errors()

original = builtins.max
builtins.max = lambda *args: "patched max"
shortcuts([3, 1, 2])
builtins.max = original

del builtins.min
try:
    shortcuts([3, 1, 2])
except NameError as e:
    print(type(e).__name__, e)
