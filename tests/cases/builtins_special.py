"""Builtins with a direct PyO3 spelling, and the dynamic fallback."""


def info(x):
    print(len(x), str(x), repr(x), bool(x), type(x), type(x).__name__)
    print(isinstance(x, list), isinstance(x, (str, bytes)))
    if len(x) > 2:
        print("long")
    if len(x):
        print("not empty")
    if 3 >= len(x) and isinstance(x, str):
        print("short str")
    if bool(x) and not isinstance(x, int):
        print("truthy")
    it = iter(x)
    print(next(it), abs(-len(x)), sorted(x), min(x), max(x))


info([3, 1, 2])
info("hello")
info("ab")
print(len("é"), str(1.5), repr("q"), bool(0), abs(-2.5), type(None))
print(divmod(7, 2), round(2.567, 2), sum([1, 2, 3]), list(range(3)))
