"""Locals that only ever hold a bool are Rust `bool`s (design §5)."""


class Weird:
    """Comparisons that do not return bool, and a truth value that counts."""

    def __init__(self, name):
        self.name = name

    def __eq__(self, other):
        return f"{self.name} == {other}"

    def __lt__(self, other):
        return [self.name, other]

    def __bool__(self):
        print("bool of", self.name)
        return True

    def __contains__(self, item):
        return item  # `in` turns this into a bool

    __hash__ = object.__hash__


def find(items, target):
    found = False
    for item in items:
        if item is target or item in (target,):
            found = True
            break
    if not found:
        print("not found", target)
    return found


def flags(x, y):
    a = isinstance(x, int)
    b = not y
    c = a and b
    d = a or b or bool(y)
    e = x is None
    f = x is not None
    g = c if e else d
    h = "a" in Weird("container")
    print(a, b, c, d, e, f, g, h)
    print([a, b], {"c": c}, (d,), f"{e!r} {f}", type(g).__name__, h == 1)
    return a, b, c, d, e, f, g, h


def not_bools(w):
    # The value of a rich comparison is whatever `__eq__` returned.
    eq = w == 1
    lt = w < 2
    print(eq, lt)
    # A truth test of a non-bool object calls `__bool__` where Python does.
    as_bool = bool(w)
    negated = not w
    print(as_bool, negated)
    # Assigned something else too: stays an object.
    mixed = False
    if as_bool:
        mixed = w
    print(mixed.name if mixed else mixed)


def shadowed(isinstance):
    # Not the builtin: the result is whatever the call returns.
    r = isinstance(1, int)
    return r


def flip(n):
    on = True
    count = 0
    while n > 0:
        on = not on
        if on:
            count += 1
        n -= 1
    print("flipped", on, count)
    done = on is True
    return done


def captured():
    # Captured by a nested function: stays a Python object in a cell.
    seen = False

    def mark():
        nonlocal seen
        seen = True

    mark()
    return seen


def maybe_unbound(flag):
    if flag:
        ok = True
    try:
        return ok
    except UnboundLocalError as err:
        print("UnboundLocalError:", err)
        return None


def in_try(values):
    failed = False
    try:
        int(values[0])
        good = True
    except ValueError:
        failed = True
        good = False
    finally:
        print("finally", failed)
    return good, failed


def in_with(lock):
    held = False
    with lock:
        held = True
    return held


def redundant(c):
    # Python code the generated Rust keeps as it is (clippy is told so).
    a = True if c else False
    b = False if c else True
    done = False
    n = 0
    while not done:
        n += 1
        if n > 2:
            break
    same = a and a
    never = a and not a
    return a, b, same, never, n


class Lock:
    def __enter__(self):
        return self

    def __exit__(self, *exc):
        return False


print(find([1, 2, 3], 2), find([1, 2, 3], 5))
flags(1, 0)
flags(None, "y")
not_bools(Weird("w"))
print(shadowed(lambda a, b: "called"))
print(flip(3), flip(0))
print(captured())
print(maybe_unbound(True), maybe_unbound(False))
print(in_try(["1"]), in_try(["x"]))
print(in_with(Lock()))
print(redundant(1), redundant(0))
