"""Lists, tuples, dicts and sets, subscripts, slices, attributes and del."""

import types


def build(a, b):
    items = [a, b, a + b]
    pair = (a, b)
    single = (a,)
    empty = ()
    lookup = {"a": a, "b": b, a: "first"}
    unique = {a, b, a}
    print(items, pair, single, empty, lookup, sorted(unique), [], {})
    print([1, "two", 3.0, None, True], (1, 2), {1: "one", 2: "two"}, {"x"})
    return items


def subscripts(items):
    print(items[0], items[-1], items[1:], items[:2], items[::2], items[::-1], items[1:-1:1])
    items[0] = "zero"
    items[1:2] = ["one", "uno"]
    del items[-1]
    del items[:1]
    print(items)
    table = {"k": 1}
    table["k"] = table["k"] + 1
    table["new"] = []
    table["new"].append(3)
    del table["k"]
    print(table)
    grid = {}
    grid[1, 2] = "cell"
    print(grid[1, 2], grid)


def attributes():
    ns = types.SimpleNamespace()
    ns.x = 1
    ns.y = ns.x + 1
    ns.x = ns.y * 10
    print(ns.x, ns.y)
    del ns.y
    print(hasattr(ns, "y"), ns)
    print("a b".split(), "abc".upper().lower())


def deletes(flag):
    value = 1
    print(value)
    del value
    if flag:
        value = 2
    other = 3
    del other
    return value


def slicer(seq, start, stop):
    return seq[start:stop], seq[start:], seq[:stop]


build(1, 2)
subscripts([1, 2, 3, 4, 5])
attributes()
print(deletes(True))
print(slicer("abcdef", 1, 3), slicer([1, 2, 3], -2, None))
glob = 5
del glob
print(deletes(False))
