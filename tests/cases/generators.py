"""Generator functions lowered to native `async fn`s (design §8)."""

import contextlib
import sys


def count(n):
    i = 0
    while i < n:
        yield i
        i += 1


def averager():
    total = 0
    n = 0
    average = None
    while True:
        value = yield average
        if value is None:
            return total
        total += value
        n += 1
        average = total / n


def bare():
    yield
    x = yield
    print("bare received", x)


def with_cleanup(log):
    try:
        yield "first"
        yield "second"
    finally:
        log.append("cleanup")


def catches():
    while True:
        try:
            yield "ready"
        except ValueError as e:
            print("caught", e, repr(sys.exception()))
            yield "recovered"


def in_loops(rows):
    for row in rows:
        if row is None:
            continue
        for item in row:
            if item == "stop":
                break
            if item == "end":
                return "ended"
            yield item
        else:
            yield "row done"
    return "all done"


def in_with(manager):
    with manager as value:
        yield value
        yield value * 2


class Manager:
    def __init__(self, value):
        self.value = value

    def __enter__(self):
        print("enter", self.value)
        return self.value

    def __exit__(self, typ, exc, tb):
        print("exit", typ.__name__ if typ else None)
        return typ is KeyError


def python_delegate():
    result = yield from iter([1, 2])
    print("python delegate result", result)
    result = yield from averager()
    return result


def native_delegate():
    total = yield from averager()
    print("native delegate total", total)
    rest = yield from count(2)
    print("count returned", rest)
    yield from in_loops([[1, "end"]])
    return total


def tree(node):
    """Recursive `yield from` of the same native generator."""
    if isinstance(node, list):
        for child in node:
            yield from tree(child)
    else:
        yield node


def ping(n):
    if n:
        yield ("ping", n)
        yield from pong(n - 1)


def pong(n):
    if n:
        yield ("pong", n)
        yield from ping(n - 1)


class Bag:
    def __init__(self, *items):
        self.items = items

    def __iter__(self):
        for item in self.items:
            yield item

    def pairs(self):
        previous = None
        for item in self:
            if previous is not None:
                yield previous, item
            previous = item

    @property
    def doubled(self):
        return (x * 2 for x in self)


class Sub(Bag):
    def __iter__(self):
        yield "sub"
        yield from super().__iter__()


@contextlib.contextmanager
def tag(name):
    print(f"<{name}>")
    try:
        yield name.upper()
    except ZeroDivisionError:
        print("contextmanager caught")
    finally:
        print(f"</{name}>")


def make_counter(start):
    count = start

    def counter():
        nonlocal count
        while True:
            count += 1
            yield count

    return counter()


def closure_gen(values):
    scale = 10
    return (lambda: [v * scale for v in values])()


def gen_with_nested():
    def inner(x):
        yield x
        yield x + 1

    for value in inner(5):
        yield value


print(list(count(4)))
g = averager()
print(next(g), g.send(10), g.send(20))
try:
    g.send(None)
except StopIteration as e:
    print("returned", e.value)

b = bare()
print(next(b), next(b))
try:
    b.send("hello")
except StopIteration:
    print("bare done")

log = []
c = with_cleanup(log)
print(next(c))
c.close()
print("closed", log)
c = with_cleanup(log)
print(next(c), next(c))
try:
    next(c)
except StopIteration:
    print("exhausted", log)

r = catches()
print(next(r))
print(r.throw(ValueError("boom")))
print(next(r))
try:
    r.throw(KeyError("other"))
except KeyError as e:
    print("propagated", repr(e))
print(repr(sys.exception()))

loops = in_loops([[1, 2], None, [3, "stop", 4], [5]])
print(list(loops))
try:
    next(loops)
except StopIteration as e:
    print("loops stopped", e.value)
print(list(in_loops([[1, "end", 2]])))

w = in_with(Manager(3))
print(list(w))
w = in_with(Manager(4))
print(next(w))
w.close()
w = in_with(Manager(5))
next(w)
try:
    w.throw(KeyError("suppressed"))
except StopIteration:
    print("suppressed by __exit__")

p = python_delegate()
print(next(p), next(p), next(p), p.send(4), p.send(6))
try:
    p.send(None)
except StopIteration as e:
    print("python_delegate returned", e.value)

n = native_delegate()
print(next(n), n.send(1), n.send(3))
print(n.send(None), next(n), next(n))
try:
    next(n)
except StopIteration as e:
    print("native_delegate returned", e.value)

print(list(tree([1, [2, [3, 4]], [], 5])))
print(list(ping(4)))

bag = Bag(1, 2, 3)
print(list(bag), list(bag.pairs()), list(bag.doubled))
print(list(Sub(7, 8)))

with tag("a") as name:
    print("inside", name)
with tag("b"):
    1 / 0
print("after b")

counter = make_counter(5)
print(next(counter), next(counter))
print(closure_gen([1, 2]))
print(list(gen_with_nested()))

gen = count(2)
print(type(gen).__name__, gen.__name__, gen.__qualname__)
print(next(gen), gen.gi_running, gen.gi_suspended)
print(list(gen), gen.gi_suspended)
