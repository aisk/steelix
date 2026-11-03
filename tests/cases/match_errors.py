# Every error a `match` can raise at run time, with CPython's messages.

import collections.abc


def attempt(label, function, value):
    try:
        print(label, function(value))
    except Exception as err:
        print(label, f"{type(err).__name__}: {err}")


class NoArgs:
    pass


class BadArgs:
    __match_args__ = ["x"]


class NotStrings:
    __match_args__ = ("x", 1)

    def __init__(self):
        self.x = 0


class Dup:
    __match_args__ = ("x",)

    def __init__(self):
        self.x = 1


def not_a_class(value):
    cls = "str"
    match value:
        case cls.upper():
            return "no"
    return "unreachable"


def positional(value):
    match value:
        case NoArgs(a):
            return a


def builtin_positional(value):
    match value:
        case int(a, b):
            return a, b


def bad_match_args(value):
    match value:
        case BadArgs(x):
            return x


def match_args_elements(value):
    match value:
        case NotStrings(x, y):
            return x, y


def multiple(value):
    match value:
        case Dup(x, x=other):
            return x, other


def duplicate_keys(value):
    match value:
        case {1: a, Value.one: b}:
            return a, b


class Value:
    one = 1


class BadMapping(collections.abc.Mapping):
    """A mapping whose keys cannot be copied for `**rest`."""

    def __getitem__(self, key):
        return key

    def __len__(self):
        return 1

    def __iter__(self):
        raise RuntimeError("cannot iterate")

    def get(self, key, default=None):
        return key


def rest(value):
    match value:
        case {"a": a, **others}:
            return a, others


def failing_eq(value):
    match value:
        case [Value.one]:
            return "one"


class Explodes:
    def __eq__(self, other):
        raise ValueError("no comparing")


def unhashable(value):
    match value:
        case {Value.unhashable: _}:
            return "no"


Value.unhashable = []

attempt("not a class:", not_a_class, 1)
attempt("no __match_args__:", positional, NoArgs())
attempt("builtin:", builtin_positional, 5)
attempt("list __match_args__:", bad_match_args, BadArgs())
attempt("non-str element:", match_args_elements, NotStrings())
attempt("multiple:", multiple, Dup())
attempt("duplicate key:", duplicate_keys, {1: "a", 2: "b"})
attempt("rest:", rest, BadMapping())
attempt("__eq__ raises:", failing_eq, [Explodes()])
attempt("unhashable key:", unhashable, {"k": 1})

# Errors raised by the guard propagate as usual.
try:
    match [0]:
        case [x] if 1 / x:
            pass
except ZeroDivisionError as err:
    print("guard:", err, "x is", x)
