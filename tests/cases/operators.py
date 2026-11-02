"""Unary operators, and/or, conditional expressions and comparisons."""


def unary(x):
    print(-x, +x, ~x, not x)


def logic(a, b):
    print(a and b, a or b, not a and b, a or b and a)
    if a and not b:
        print("a only")
    elif a or b:
        print("some")
    else:
        print("none")


def choose(flag, x):
    label = "big" if x > 10 else "small"
    print(label, x if flag else -x)
    return "yes" if flag and x else "no"


def identity(x, y):
    print(x is y, x is not y, x is None, x is not None, None is x)
    if x is None:
        return "nothing"
    return "something"


def membership(item, container):
    if item in container and item not in "xyz":
        return True
    return item in container


def values(a, b):
    lt = a < b
    print(lt, a <= b, a == b, a != b, a > b, a >= b)
    print(1 < a < 5, 1 < a < b < 10, a == b == 3)
    if 0 <= a < 10:
        print("digit")
    if not 0 <= b < 10:
        print("not a digit")


unary(5)
unary(-3)
logic(0, 3)
logic(2, 0)
logic(0, 0)
logic([], "s")
print(choose(True, 20), choose(False, 0))
print(identity(None, None), identity(1, None))
print(membership("a", "abc"), membership("x", "xyz"), membership(4, [1, 2]))
values(3, 7)
values(12, 3)
values(3, 3)
nothing = None
print(nothing or "default", nothing and 1, 0 or None or "last")
print(..., nothing is ..., [...])
