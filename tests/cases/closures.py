"""Nested functions, closures and `nonlocal`."""


def make_counter(start=0):
    count = start

    def increment(step=1):
        nonlocal count
        count += step
        return count

    def peek():
        return count

    return increment, peek


def late_binding():
    # Every closure sees the final value of `i`.
    funcs = []
    for i in range(3):

        def show():
            return i

        funcs.append(show)
    return [funcs[0](), funcs[1](), funcs[2]()]


def early_binding():
    # A default argument captures the value at definition time instead.
    funcs = []
    for i in range(3):
        funcs.append(lambda i=i: i * 10)
    return [funcs[0](), funcs[1](), funcs[2]()]


def factorial(n):
    def go(k, acc):
        if k <= 1:
            return acc
        return go(k - 1, acc * k)

    return go(n, 1)


def deep():
    a = 1

    def middle():
        b = 2

        def inner():
            nonlocal a
            a += b
            return a

        return inner

    return middle()


def unassigned():
    def read():
        return value

    try_first = read
    value = "now set"
    return try_first()


def too_early():
    def read():
        return secret

    result = read()
    secret = 1
    return result


increment, peek = make_counter(10)
print(increment(), increment(5), peek())
other, _ = make_counter()
print(other(), increment(), peek())
print(late_binding(), early_binding())
print(factorial(10))
inner = deep()
print(inner(), inner())
print(unassigned())
print(increment.__qualname__, increment.__name__, peek.__closure__ is not None)
print(type(increment.__closure__[0]).__name__, increment.__closure__[0].cell_contents)
too_early()
