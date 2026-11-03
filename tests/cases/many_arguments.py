"""Calls with more than 12 positional arguments: PyO3's tuples stop at 12,
so the statement is kept as Python (design §13), also for the builtins that
are otherwise called directly (`print`, a raised builtin exception)."""


def show(*args):
    return len(args)


def f():
    print(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12)
    print(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13)
    print(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, sep="-")
    print(show(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14))
    try:
        raise ValueError(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13)
    except ValueError as e:
        print(len(e.args))


f()
print(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13)
