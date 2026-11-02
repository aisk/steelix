"""List, set and dict comprehensions run inline (PEP 709): their variables
do not leak, the first iterable is evaluated outside, and errors are
reported in the enclosing frame on the line of the failing part."""

import traceback
import types


def show(exc):
    print(type(exc).__name__, exc)
    for frame in traceback.extract_tb(exc.__traceback__):
        print(" ", frame.name, frame.lineno, frame.line)


def squares(xs):
    return [x * x for x in xs]


def evens(d):
    return {k: v for k, v in d.items() if v % 2 == 0}


def letters(words):
    return {c for w in words for c in w if c.isalpha() if c != "z"}


def pairs(n):
    return [(i, j) for i in range(n) for j in range(i) if (i + j) % 2]


def nested(rows):
    # The inner comprehension sees the variable of the outer one.
    return [[cell * row[0] for cell in row] for row in rows]


def unpacking(items):
    return [(a, rest, b) for a, *rest, b in items]


def attribute_target():
    box = types.SimpleNamespace()
    values = [box.value for box.value in range(3)]
    return values, box.value


def subscript_target():
    slots = [0]
    return [slots[0] for slots[0] in "ab"], slots


def dict_order():
    def key(k):
        print("key", k)
        return k % 2

    def value(v):
        print("value", v)
        return v

    # The key is evaluated before the value; a later duplicate key wins.
    return {key(k): value(k) for k in range(4)}


x = "global x"


def shadow():
    squares = [x * x for x in range(3)]
    return squares, x


def first_iterable_outside():
    x = [1, 2]
    # The first iterable is the function's `x`, then `x` is the item.
    return [x * 10 for x in x]


def late_binding():
    funcs = [lambda: i for i in range(3)]
    return [f() for f in funcs]


def default_binding():
    funcs = [lambda i=i: i for i in range(3)]
    return [f() for f in funcs]


def generator_in_comprehension():
    return [list(x + y for y in range(2)) for x in (10, 20)]


def stop_iteration():
    def boom(v):
        if v == 2:
            raise StopIteration("not converted in a comprehension")
        return v

    try:
        print([boom(v) for v in range(4)])
    except StopIteration as exc:
        print("StopIteration", exc)


def multiline(xs):
    return [
        10 // x
        for x in xs
        if x != 1
    ]


def failing_condition(xs):
    return [x for x in xs
            if 10 // x]


class Bad:
    def __iter__(self):
        return self

    def __next__(self):
        raise ValueError("next failed")


def failing_iteration():
    return [x for x in
            Bad()]


class Config:
    size = 3
    doubled = [i * 2 for i in range(size)]
    try:
        broken = [size for i in range(2)]
    except NameError as exc:
        print("class body:", exc)
    names = {n: len(n) for n in ("a", "bb")}


class Base:
    def items(self):
        return [1, 2]


class Derived(Base):
    def __secret(self):
        return "secret"

    def values(self):
        return [v * 2 for v in super().items()] + [__class__.__name__ for _ in "x"]

    def secrets(self):
        return [self.__secret() for _ in range(2)]

    def supers(self):
        return [super().items() for _ in "x"]


print(squares([1, 2, 3]), evens({"a": 1, "b": 2, "c": 4}))
print(sorted(letters(["ab1", "zc"])), pairs(4), nested([[1, 2], [3, 4]]))
print(unpacking([(1, 2, 3, 4), (5, 6)]), attribute_target(), subscript_target())
print(dict_order())
print(shadow(), x, first_iterable_outside())
print(late_binding(), default_binding(), generator_in_comprehension())
stop_iteration()
for call, arg in ((multiline, [1, 2, 0]), (failing_condition, [1, 0])):
    try:
        call(arg)
    except ZeroDivisionError as exc:
        show(exc)
try:
    failing_iteration()
except ValueError as exc:
    show(exc)
print(Config.doubled, Config.names, Config.size)
d = Derived()
print(d.values(), d.secrets(), d.supers())
module_level = [n for n in range(3) if n]
print(module_level, [[c for c in w] for w in ("ab", "c")], {n % 2 for n in range(5)})
print("n" in globals(), {k: v for k, v in zip("ab", range(2))})
