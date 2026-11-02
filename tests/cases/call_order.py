"""Evaluation order of call arguments with keywords, `*` and `**`."""


def show(label, value=None):
    print("eval", label)
    return value


def target(*args, **kwargs):
    return [args, kwargs]


def fixed(a, b, c=3, *, d=4):
    return [a, b, c, d]


print(target(show("a", 1), *show("star", [2, 3]), show("b", 4), k=show("k", 5), **show("kw", {"z": 6})))
print(target(show("x", 1), y=show("y", 2), *show("late star", [3])))
print(fixed(show("1", 1), c=show("c", 30), b=show("b", 20)))
print(fixed(*show("args", [1, 2]), **show("kwargs", {"d": 40})))
print(show("callee", fixed)(show("arg", 1), show("arg", 2)))
print(target(**{"a": 1}, **{"b": 2}), target(*"ab", *(1, 2)))
