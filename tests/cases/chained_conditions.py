"""Chained comparisons in conditions whose operands need temporaries."""


def f(x):
    print("f", x)
    return x


def check(a, b, d):
    if a < b < f(3) < d:
        print("all ascending")
    else:
        print("not ascending")
    if a < f(b) < f(3) <= d < f(10):
        print("long chain")
    if f(a) == 1 < f(b) and b < f(5) < d:
        print("mixed")
    elif a < f(0) < d:
        print("elif")
    n = 0
    while n < f(2) < d:
        n += 1
    print("n", n)


check(1, 2, 4)
check(1, 2, 3)
check(5, 2, 4)
