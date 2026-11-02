"""Lambda expressions."""

square = lambda x: x * x
add = lambda a, b=10, *rest, scale=1, **extra: (a + b + len(rest) + len(extra)) * scale


def make_adder(n):
    return lambda x: x + n


words = ["banana", "Apple", "cherry"]
print(square(7), add(1), add(1, 2, 3, 4, scale=2, k=0))
print(make_adder(5)(10), sorted(words, key=lambda w: w.lower()))
print(square.__name__, square.__qualname__, make_adder(1).__qualname__)
print((lambda: "called at once")())
print(list(map(lambda p: p[0] * p[1], [(1, 2), (3, 4)])))
