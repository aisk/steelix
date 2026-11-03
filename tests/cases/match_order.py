# The order of everything a `match` does (design §8), observed by objects
# that print each call: `len()`, iteration and indexing of sequences, `.get()`
# of mappings, `__eq__` of value patterns, `isinstance` and attributes of
# class patterns, and when names are stored.

import collections.abc


def log(*args):
    print("  ", *args)


class Seq(collections.abc.Sequence):
    def __init__(self, *items):
        self.items = items

    def __len__(self):
        log("len")
        return len(self.items)

    def __getitem__(self, index):
        log("getitem", index)
        return self.items[index]

    def __iter__(self):
        log("iter")
        for item in self.items:
            log("next", item)
            yield item


class Eq:
    def __init__(self, name, result=True):
        self.name = name
        self.result = result

    def __eq__(self, other):
        log("eq", self.name, other)
        return self.result

    def __bool__(self):
        log("bool", self.name)
        return True

    def __repr__(self):
        return self.name


class Map(collections.abc.Mapping):
    def __init__(self, **items):
        self.items = items

    def __len__(self):
        log("len")
        return len(self.items)

    def __getitem__(self, key):
        log("getitem", key)
        return self.items[key]

    def __iter__(self):
        log("iter")
        return iter(self.items)

    def get(self, key, default=None):
        log("get", key)
        return self.items.get(key, default)

    def keys(self):
        log("keys")
        return self.items.keys()


class Values:
    """Value patterns are evaluated when the pattern gets to them."""

    def __getattr__(self, name):
        log("value", name)
        return name


values = Values()


class Meta(type):
    def __instancecheck__(cls, instance):
        log("isinstance", cls.__name__)
        return super().__instancecheck__(instance)

    @property
    def __match_args__(cls):
        log("__match_args__")
        return ("a", "b")


class Obj(metaclass=Meta):
    def __init__(self, **attrs):
        self.__dict__.update(attrs)

    def __getattr__(self, name):
        log("getattr", name, "(missing)")
        raise AttributeError(name)

    def __getattribute__(self, name):
        if not name.startswith("__"):
            log("getattribute", name)
        return super().__getattribute__(name)


class Stored(dict):
    """A class namespace that shows when names are stored."""

    def __setitem__(self, key, value):
        if not key.startswith("__"):
            log("store", key, value)
        super().__setitem__(key, value)


class StoreMeta(type):
    @classmethod
    def __prepare__(mcs, name, bases):
        return Stored()


def sequences():
    print("unpacked by iteration")
    match Seq(1, 2, 3):
        case [1, x, 4]:
            print("no")
        case [a, b, c] if log("guard", a, b, c):
            print("no")
        case [a, *rest]:
            print("match", a, rest)
    print("read by index around *_")
    match Seq(1, 2, 3, 4):
        case [0, *_, 4]:
            print("no")
        case [first, *_, Eq("third", False), last]:
            print("no")
        case [first, _, *_, last]:
            print("match", first, last)
    print("only wildcards")
    match Seq(1, 2):
        case [_, _, _]:
            print("no")
        case [*_]:
            print("star only: no len()")
    match Seq(1, 2):
        case [_, *_]:
            print("at least one")


def values_and_eq():
    print("value patterns")
    match Eq("subject", False):
        case 1:
            print("no")
        case values.first | values.second:
            print("no")
        case [values.never]:
            print("no")
        case None:
            print("no")
        case _:
            print("nothing matched")
    match 5:
        case Eq("pattern"):
            print("no: Eq() is a class pattern")
        case _:
            pass
    match [5]:
        case [values.five] | [5]:
            print("matched [5]")


def mappings():
    print("mapping patterns")
    match Map(a=1, b=2, c=3):
        case {"x": 1, "y": 2, "z": 3, "w": 4}:
            print("no: too few items, no lookups")
        case {"a": 1, "missing": _, "b": 2}:
            print("no")
        case {"a": Eq("one", False)}:
            print("no")
        case {values.a: 1, "b": b, **rest}:
            print("match", b, sorted(rest))
    match [1]:
        case {values.not_evaluated: _}:
            print("no")
        case _:
            print("keys only after the mapping check")


def classes():
    print("class patterns")
    obj = Obj(a=1, b=2, c=3)
    match obj:
        case Obj(1, 2, d=_):
            print("no")
        case Obj(Eq("first", False), c=3):
            print("no")
        case Obj(a, b, c=c) if log("guard", a, b, c):
            print("no")
        case Obj(c=3, a=1):
            print("match")
    match 1:
        case Obj():
            print("no")
        case int():
            print("an int")


sequences()
values_and_eq()
mappings()
classes()
print("captures are stored after the whole pattern, in order")


class Body(metaclass=StoreMeta):
    match [1, (2, 3), {"k": 4, "j": 5}]:
        case [a, (b, c) as pair, {"k": d, **rest}] as whole if log("guard"):
            pass
        case [a, *more]:
            log("body")


