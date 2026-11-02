"""Methods on displays and `*args`/`**kwargs` behave like on any object.

The concrete PyO3 types have their own `get_item`, `len`, .. with other
semantics; a missing dict key must raise `KeyError`, not give `None`.
"""


def sizes(*args, **kwargs):
    print(len(args), len(kwargs), args[-1], kwargs["k"])
    print("k" in kwargs, 1 in args, args[1:])


def containers():
    print(len([1, 2]), len((1, 2, 3)), len({1, 2}), len({"a": 1}), len(()))
    print([1, 2, 3][-1], [1, 2, 3][1:], (1, 2)[0], {"a": 1}["a"])
    print(2 in [1, 2], 3 in (1, 2), "a" in {"a": 1}, 1 in {1})
    print([1, 2] + [3], (1,) * 2, {1, 2} | {3}, {1, 2} - {1})
    if [1, 2]:
        print("non-empty list")
    if not {}:
        print("empty dict")
    xs = [1, 2, 3]
    if iter(xs) == iter(xs):
        print("unreachable")
    print(iter(xs) != iter(xs), str(1) == "1", type(1) == int)


def lookup(key):
    print({"a": 1}["a"])
    print({"a": 1}[key])
    print("unreachable in CPython")


sizes(1, 2, 3, k="v")
containers()
lookup("b")
