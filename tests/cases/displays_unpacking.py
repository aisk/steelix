# `*` and `**` in list, tuple, set and dict displays.


def trace(label, value):
    print("eval", label)
    return value


class Noisy:
    """An iterable that reports when it is iterated."""

    def __init__(self, label, items):
        self.label = label
        self.items = items

    def __iter__(self):
        print("iter", self.label)
        return iter(self.items)


class Key:
    """A key that reports when it is hashed."""

    def __init__(self, name):
        self.name = name

    def __hash__(self):
        print("hash", self.name)
        return hash(self.name)

    def __eq__(self, other):
        return isinstance(other, Key) and other.name == self.name

    def __repr__(self):
        return f"Key({self.name!r})"


class Mapping:
    def keys(self):
        return ["m1", "m2"]

    def __getitem__(self, key):
        return key.upper()


def lists(a, b):
    first = [*a, b]
    second = [trace("x", 0), *trace("a", Noisy("a", a)), trace("b", b), *Noisy("c", "cd")]
    nested = [*[*a], *(), *range(2)]
    # The comprehension needs statements first: they run after `Noisy` is iterated.
    lazy = [*Noisy("g", "g"), [trace("comp", i) for i in range(1)], *[i for i in a]]
    print(first, second, nested, lazy)
    return [*a]


def tuples(a):
    one = (*a,)
    print(one, (*one, *a, 9), type(one).__name__)


def sets(a):
    s = {trace("k1", Key("k1")), trace("k2", Key("k2")), *trace("a", Noisy("a", [Key("k3")]))}
    print(sorted(k.name for k in s), {*a} == set(a), {1, *a})


def dicts(d, v):
    merged = {**d, "k": v}
    order = {"a": 1, "b": 2, **{"a": 3, "c": 4}, "b": 5}
    keys = {trace("k1", Key("k1")): trace("v1", 1), trace("k2", Key("k2")): 2, **d}
    print(merged, order, list(order), keys)
    print({**Mapping(), "x": 1}, {**{}}, {**d, **d})


def errors():
    for build in (
        lambda: [*1],
        lambda: (*None,),
        lambda: {*3.5},
        lambda: {**1},
        lambda: {"a": 1, **[("b", 2)]},
        lambda: [0, *Noisy("bad", 5)],
    ):
        try:
            build()
        except TypeError as e:
            print(type(e).__name__, e)


lists([1, 2], 3)
tuples("ab")
sets([1, 2])
dicts({"x": 1}, 2)
errors()
print(*[1, 2], *"ab", sep="-")
values = [*range(3), *"xy"]
print(values, (*values,), {**{"m": 1}})
