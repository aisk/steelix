"""Generator expressions are lazy: only the first iterable is evaluated
when the generator is created, the rest runs as items are requested, in
a `<genexpr>` frame of its own."""

import gc
import itertools
import traceback
import weakref


def show(exc):
    print(type(exc).__name__, exc)
    for frame in traceback.extract_tb(exc.__traceback__):
        print(" ", frame.name, frame.lineno, frame.line)


def noisy(x):
    print("check", x)
    return x > 2


def short_circuit(xs):
    return any(noisy(x) for x in xs)


def infinite():
    gen = (n * n for n in itertools.count())
    first = next(gen)
    rest = list(itertools.islice(gen, 3))
    return first, rest


def late_binding():
    factor = 1
    gen = (x * factor for x in range(3))
    factor = 10
    return list(gen)


def first_iterable_now():
    print("creating")
    gen = (x for x in noisy_iterable())
    print("created")
    return list(gen)


def noisy_iterable():
    print("first iterable evaluated")
    return [1, 2]


def failing(xs):
    gen = (10 // x for x in xs)
    print("created")
    out = []
    for value in gen:
        out.append(value)
    return out


class Bad:
    def __iter__(self):
        return self

    def __next__(self):
        raise ValueError("next failed")


def failing_iteration():
    gen = (x for x in
           Bad())
    return sum(gen)


def failing_condition(xs):
    return list(x for x in xs if 10 // x)


def closed_early():
    def source():
        try:
            yield 1
            yield 2
            yield 3
        finally:
            print("source closed")

    gen = (x * 2 for x in source())
    for value in gen:
        print("got", value)
        break
    gen.close()
    print("after close", list(gen))
    other = (x for x in source())
    print(next(other))
    del other
    print("deleted")


def stop_iteration():
    def boom(v):
        if v == 2:
            raise StopIteration("inner")
        return v

    try:
        print(list(boom(v) for v in range(4)))
    except RuntimeError as exc:
        print(type(exc).__name__, exc, repr(exc.__cause__), exc.__suppress_context__)


def protocol():
    g = (i for i in range(3))
    print(type(g).__name__, repr(g).startswith("<generator object protocol.<locals>.<genexpr> at "))
    print(g.__name__, g.__qualname__, iter(g) is g, g.gi_running)
    try:
        g.send(1)
    except TypeError as exc:
        print("TypeError", exc)
    print(g.send(None), next(g))
    try:
        g.throw(ValueError("thrown"))
    except ValueError as exc:
        print("thrown back", exc)
    print(list(g), next(g, "exhausted"))
    g.__name__ = "renamed"
    print(g.__name__)


def consumers(words):
    print(sum(len(w) for w in words), "-".join(w.upper() for w in words))
    print(sorted((w for w in words), key=len), max((w for w in words), key=len))
    print(tuple(w[0] for w in words), dict((w, len(w)) for w in words))
    print(all(w for w in words), set(w[-1] for w in words) == {"a", "c"})
    print(list((a, b) for a, b in zip(words, range(3))))


def captures():
    base = 10

    def inner():
        return list(base + i for i in range(2))

    return inner()


def comprehension_variable():
    # Each generator sees the comprehension's variable when it runs.
    gens = [(x for _ in range(1)) for x in range(3)]
    return [list(g) for g in gens]


def collected():
    # The generator is in a reference cycle through the cell of `g`.
    g = None
    g = (x for x in range(3) if g is not None)
    ref = weakref.ref(g)
    del g
    gc.collect()
    return ref() is None


def reentrant():
    def inner(x):
        return next(g)

    g = (inner(x) for x in range(3))
    try:
        next(g)
    except ValueError as exc:
        return str(exc)


class Table:
    size = 3
    rows = list((i, i) for i in range(size))
    try:
        broken = list(size for i in range(2))
    except NameError as exc:
        print("class body:", exc)


print(short_circuit([1, 3, 5]), infinite(), late_binding())
print(first_iterable_now())
try:
    failing([1, 0])
except ZeroDivisionError as exc:
    show(exc)
try:
    failing_iteration()
except ValueError as exc:
    show(exc)
try:
    failing_condition([1, 0])
except ZeroDivisionError as exc:
    show(exc)
closed_early()
stop_iteration()
protocol()
consumers(["a", "bb", "abc"])
print(captures(), comprehension_variable(), Table.rows, collected(), reentrant())
module_gen = (c * 2 for c in "ab")
print(next(module_gen), list(module_gen), type(module_gen).__qualname__)
