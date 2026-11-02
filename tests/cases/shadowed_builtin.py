"""A builtin rebound at module level is no longer special-cased."""


def len(x):
    return 42


def use(xs):
    return len(xs) + 1


str = repr
print(use([1, 2]), len("abc"), str("quoted"))
