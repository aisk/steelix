"""Locals that only ever hold a list, dict or set display are typed Rust
locals (`Bound<'py, PyList>`, ..), and `xs.append(x)`, `s.add(x)` call
PyO3's methods. Everything else about them stays what Python does. Locals
that may hold something else stay objects, and so do those that are
subscripted (`xs[i]`, `d[k]`, `xs[i] = v`), which reads better on an object.
"""

import asyncio


class Loud:
    def __init__(self, name):
        self.name = name

    def __repr__(self):
        return f"Loud({self.name})"

    def __del__(self):
        print("del", self.name)


def report(f, *args):
    try:
        print(f.__name__, "->", f(*args))
    except Exception as e:
        print(f.__name__, "raised", type(e).__name__, e)


# --- qualifying locals ---


def build(n):
    xs = []
    d = {}
    s = {0}
    for i in range(n):
        xs.append(i)
        xs.append(i * i)
        d[i] = str(i)
        s.add(i % 3)
    print(len(xs), len(d), len(s), 2 in xs, 7 in d, 1 in s, 9 not in s)
    print(xs[0], xs[-1], xs[1:3], d[0], d.get(99), d.get(99, "none"))
    print(sorted(s), sorted(d.items()), xs.index(4), xs.count(1))
    xs[0] = "first"
    d["k"] = xs
    del d[0]
    del xs[1]
    xs.extend([7, 8])
    xs.insert(0, "zero")
    print(xs.pop(), d.pop("k") is xs, s.pop() in (0, 1, 2), xs)
    xs.sort(key=str)
    xs.reverse()
    s.discard(1)
    s.update([5, 6])
    d.update(a=1)
    d.setdefault("b", []).append(2)
    return xs, d, sorted(s)


def comprehensions(n):
    squares = [i * i for i in range(n)]
    by_value = {i * i: i for i in range(n)}
    odd = {i for i in range(n) if i % 2}
    squares.append(-1)
    odd.add(-1)
    by_value[-1] = None
    nested = [[j for j in range(i)] for i in squares if i >= 0]
    return squares, by_value, sorted(odd), nested, [x for x in squares if x in odd]


def unpacking_displays(a, b):
    joined = [*a, *b]
    merged = {**b, "z": 26}
    union = {*a, *b}
    joined.append(len(merged))
    union.add("u")
    return joined, merged, sorted(union)


def aliases():
    xs = [1]
    ys = xs
    zs = ys
    zs.append(2)
    ys.append(3)
    print(xs is ys, ys is zs, xs)
    ys = [9]
    ys.append(10)
    xs = ys
    xs.append(11)
    return xs, ys, zs


def branches(flag):
    if flag:
        out = []
    else:
        out = ["else"]
    out.append(flag)
    while len(out) < 4:
        out = [*out, len(out)]
    return out


def used_as_values(xs_in):
    xs = [3, 1, 2]
    d = {"sep": "-", "end": "!\n"}
    s = {1}
    print(*xs, **d)
    print(f"{xs} {d!r} {s!s:>5} {len(xs):03}")
    print(str(xs), repr(d), bool(xs), not s, xs == [3, 1, 2], xs != xs_in, xs < [4])
    print(xs + [4], xs * 2, s | {2}, s & {1}, d | {"x": 1}, type(xs) is list, type(s).__name__)
    print(isinstance(xs, list), isinstance(d, (list, dict)), hash(tuple(xs)) == hash((3, 1, 2)))
    print(sorted(xs), max(xs), sum(xs), list(reversed(xs)), dict(d), set(s), tuple(xs))
    a, b, c = xs
    first, *rest = xs
    print(a, b, c, first, rest, xs if xs else None, xs or None, xs and d)
    holder = [xs, d, s]
    pair = (xs, s)
    mapping = {"xs": xs, 1: d}
    print(holder, pair, mapping, holder[0] is xs)
    return xs


def echo(*args, **kwargs):
    return args, kwargs


def passing():
    xs = [1, 2]
    d = {"k": "v"}
    print(echo(xs, d), echo(*xs, **d), echo(xs, key=d), echo(xs=xs))
    print(takes(xs, d), takes(d, xs))
    it = iter(xs)
    print(next(it), next(it), next(it, "done"))
    return d


def takes(a, b):
    a_is = type(a).__name__
    return a_is, len(b)


def iteration():
    xs = [1, 2, 3]
    seen = []
    for x in xs:
        seen.append(x)
        if x < 3 and len(xs) < 6:
            xs.append(x * 10)
    print(seen, xs)
    s = {1, 2}
    try:
        for x in s:
            s.add(x + 10)
    except RuntimeError as e:
        print("RuntimeError", e)
    d = {1: "a"}
    try:
        for k in d:
            d[k + 1] = "b"
    except RuntimeError as e:
        print("RuntimeError", e)
    for i, x in enumerate(xs):
        if x == 2:
            del xs[i]
    return xs, sorted(s), d


def lifetimes():
    xs = [Loud("a")]
    xs.append(Loud("b"))
    print("rebinding")
    xs = [Loud("c")]
    print("rebound")
    d = {"k": Loud("d")}
    d["k"] = Loud("e")
    print("replaced")
    s = {1}
    s.add(Loud("f"))
    ys = xs
    xs = []
    print("still alive", ys)
    ys = xs
    print("gone")
    xs.append(Loud("g"))
    return len(xs) + len(d) + len(s)


def evaluation_order():
    xs = []

    def note(x):
        print("note", x, len(seen))
        seen.append(x)
        return x

    seen = []
    xs.append(note(1))
    xs.append(note(xs))
    d = {}
    d[note("key")] = note("value")
    s = set()
    s.add(note("s"))
    return len(xs), d, s, seen


def attempt(label, f):
    try:
        print(label, f())
    except Exception as e:
        print(label, type(e).__name__, e)


def protocols(flag):
    xs = [1, 2]
    ys = [3]
    d = {"k": 1}
    s = {1}
    print(xs is None, xs is not ys, xs == ys, [1, 2] == xs != ys, xs < ys <= ys, d == {"k": 1})
    print(not xs, not not d, xs and ys, s or xs, xs if flag else d, len(xs) > 1, len(s) == len(d))
    print(xs.__len__(), xs.__class__.__name__, d.keys() & s, s.__contains__(1), d.__doc__[:4])
    print([xs for _ in range(2)], {len(v): v for v in (xs, ys)}, (n := xs) is xs, n)
    print(f"{xs!a} {d!r:>12} {s}", "%s %r" % (xs, d), str(xs) + repr(s))  # noqa: UP031
    while ys:
        print("pop", ys.pop())
    if d:
        print("dict is true")
    assert xs, d
    for label, f in [("call", lambda: 0)]:
        attempt(label, f)
    try:
        xs()
    except TypeError as e:
        print("TypeError", e)
    try:
        print(xs.missing)
    except AttributeError as e:
        print("AttributeError", e)
    try:
        d.attr = 1
    except AttributeError as e:
        print("AttributeError", e)
    try:
        del s.attr
    except AttributeError as e:
        print("AttributeError", e)
    try:
        print(-xs)
    except TypeError as e:
        print("TypeError", e)
    try:
        print(s + 1)
    except TypeError as e:
        print("TypeError", e)
    try:
        print(xs @ d, xs - s)
    except TypeError as e:
        print("TypeError", e)
    try:
        print({xs: 1})
    except TypeError as e:
        print("TypeError", e)
    try:
        print(isinstance(1, xs))
    except TypeError as e:
        print("TypeError", e)
    try:
        raise xs
    except TypeError as e:
        print("TypeError", e)
    try:
        raise ValueError("cause") from d
    except TypeError as e:
        print("TypeError", e)
    try:
        with xs:
            pass
    except TypeError as e:
        print("TypeError", e)
    try:
        assert not s, xs
    except AssertionError as e:
        print("AssertionError", e, e.args[0] is xs)
    try:
        a, b = s
    except ValueError as e:
        print("ValueError", e)
    return xs if flag else d


def returns_from_blocks(flag):
    xs = [1]
    try:
        xs.append(2)
        if flag:
            return xs
        with open(__file__):
            return xs
    finally:
        xs.append("finally")


# --- errors keep CPython's types and messages ---


def subscripted(i):
    xs = [1, 2, 3]
    d = {1: "one"}
    keys = {"a": 1}
    del keys["a"]
    keys[i] = xs
    xs[i] += 10
    print(xs[i], xs[-1], xs[i:], d[i], keys)
    del xs[0]
    return d[xs[0]]


def missing_key():
    d = {"a": 1}
    return d["b"]


def missing_index():
    xs = [1]
    return xs[5]


def bad_index():
    xs = [1]
    return xs["x"]


def unhashable_key():
    d = {}
    d[[1]] = 2
    return d


def unhashable_element():
    s = {1}
    s.add([1])
    return s


def unhashable_lookup():
    d = {}
    return [] in d


def append_arity():
    xs = []
    xs.append(1, 2)
    return xs


def append_nothing():
    xs = []
    xs.append()
    return xs


def append_keyword():
    xs = []
    xs.append(object=1)
    return xs


def append_star():
    xs = []
    args = (1,)
    xs.append(*args)
    xs.append(*())
    return xs


def add_arity():
    s = {1}
    s.add(1, 2)
    return s


def no_such_method():
    xs = []
    d = {}
    d.add(1)
    xs.add(1)
    return xs


def append_result():
    xs = []
    r = xs.append(1)
    print(xs.append(2), {3}.add(4), [xs.append(i) for i in range(2)])
    f = xs.append
    f(5)
    g = xs.append
    xs.clear()
    g(6)
    return r, xs, f.__self__ is xs, f.__name__


def delete_missing():
    d = {}
    xs = []
    try:
        del d["k"]
    except KeyError as e:
        print("KeyError", e)
    del xs[0]


def assign_missing():
    xs = []
    xs[0] = 1


# --- locals that are not typed ---


def rebound_to_other_type(flag):
    xs = []
    if flag:
        xs = ()
    try:
        xs.append(1)
    except AttributeError as e:
        print("AttributeError", e)
    d = {}
    d = d.items()
    s = {1}
    s = frozenset(s)
    try:
        s.add(2)
    except AttributeError as e:
        print("AttributeError", e)
    mixed = []
    mixed = {}
    mixed["k"] = 1
    return xs, list(d), s, mixed


class Appender(list):
    def append(self, x):
        print("Appender.append", x)
        super().append(x * 2)


def subclass_or_call(flag):
    xs = Appender() if flag else []
    xs.append(1)
    ys = list()
    ys.append(2)
    zs = [] or Appender()
    zs.append(3)
    ws = xs if flag else ys
    ws.append(4)
    return xs, ys, zs, type(ws).__name__


def chained():
    a = b = []
    a.append(1)
    b.append(2)
    c = d = {}
    c["k"] = a
    return a is b, b, c is d, d


def captured():
    xs = []
    s = set()

    def push(x):
        xs.append(x)
        s.add(x)
        return len(xs)

    push(1)
    xs.append(2)
    s.add(3)
    return xs, sorted(s), push(4)


def rebound_by_nested():
    xs = []

    def replace():
        nonlocal xs
        xs = Appender()

    xs.append(1)
    replace()
    xs.append(2)
    return xs


def maybe_unassigned(flag):
    if flag:
        xs = []
    xs.append(1)
    return xs


def deleted():
    xs = []
    xs.append(1)
    del xs
    xs = {}
    xs["k"] = 1
    return xs


def other_targets(pairs):
    xs = []
    xs, n = Appender(), 1
    xs.append(n)
    ys = []
    for ys in pairs:
        print("item", ys)
    zs = []
    if (zs := Appender()) is not None:
        zs.append(3)
    ws = []
    ws += (5,)
    ws.append(6)
    with open(__file__) as us:
        us = []
        us.append(7)
    import os as vs

    print(vs.sep)
    vs = []
    vs.append(8)
    return xs, ys, zs, ws, us, vs


def parameter(xs, d=None):
    xs = []
    xs.append(1)
    if d is None:
        d = {}
    d["k"] = xs
    return xs, d


def matching(command):
    xs = [command, 1]
    d = {"cmd": command, "n": 2}
    match xs:
        case ["go", n]:
            print("list go", n)
        case [first, *rest]:
            print("list", first, rest)
    match d:
        case {"cmd": "go", **rest}:
            print("dict go", rest)
        case {"cmd": other}:
            print("dict", other)
    match command:
        case "stop":
            found = []
        case _:
            found = [command]
    found.append(len(xs))
    match found:
        case list(items):
            items.append("matched")
    return found


def in_try(key):
    xs = []
    d = {}
    try:
        xs.append(1)
        d["a"] = xs
        ys = [d[key]]
        ys.append(2)
    except KeyError as e:
        xs.append(e.args)
        ys = []
    finally:
        xs.append("finally")
    with open(__file__) as f:
        zs = [len(f.readline()) > 0]
        zs.append(len(xs))
        xs.append("with")
    return xs, ys, zs


def generator(n):
    xs = []
    s = {-1}
    yield from xs
    for i in range(n):
        xs.append(i)
        s.add(i)
        got = yield xs
        if got:
            xs = [got]
            xs.append("sent")
    return xs, sorted(s)


def drive():
    gen = generator(3)
    print(next(gen), gen.send("x"), next(gen))
    try:
        next(gen)
    except StopIteration as e:
        print("returned", e.value)
    print(list(tuple(v) for v in generator(2)), sum(len(v) for v in generator(3)))


async def coroutine(n):
    xs = []
    d = {}
    for i in range(n):
        await asyncio.sleep(0)
        xs.append(i)
        xs.append(await asyncio.sleep(0, i))
        d[i] = await helper(i)
    try:
        await xs
    except TypeError as e:
        print("TypeError", e)
    try:
        async for x in xs:
            print(x)
    except TypeError as e:
        print("TypeError", e)
    return await asyncio.sleep(0, xs), d


async def helper(n):
    seen = []
    for i in range(n):
        seen.append(i)
        await asyncio.sleep(0)
    return seen


def lambdas_and_defaults():
    xs = [1]
    f = lambda: [].append(1)  # noqa: E731
    ys = [f()]
    ys.append((lambda v=xs: v)())

    def default(v=xs):
        return v

    xs.append(default() is xs)
    return xs, ys


total = []
total.append(1)
seen = set()
seen.add("module")
table = {}
table["k"] = total

for f, args in [
    (build, (4,)),
    (comprehensions, (5,)),
    (unpacking_displays, ("ab", {"c": 1})),
    (aliases, ()),
    (branches, (True,)),
    (branches, (False,)),
    (used_as_values, ([3, 1, 2],)),
    (passing, ()),
    (iteration, ()),
    (lifetimes, ()),
    (evaluation_order, ()),
    (protocols, (True,)),
    (protocols, (False,)),
    (returns_from_blocks, (True,)),
    (returns_from_blocks, (False,)),
    (subscripted, (1,)),
    (subscripted, (0,)),
    (missing_key, ()),
    (missing_index, ()),
    (bad_index, ()),
    (unhashable_key, ()),
    (unhashable_element, ()),
    (unhashable_lookup, ()),
    (append_arity, ()),
    (append_nothing, ()),
    (append_keyword, ()),
    (append_star, ()),
    (add_arity, ()),
    (no_such_method, ()),
    (append_result, ()),
    (delete_missing, ()),
    (assign_missing, ()),
    (rebound_to_other_type, (True,)),
    (rebound_to_other_type, (False,)),
    (subclass_or_call, (True,)),
    (subclass_or_call, (False,)),
    (chained, ()),
    (captured, ()),
    (rebound_by_nested, ()),
    (maybe_unassigned, (True,)),
    (maybe_unassigned, (False,)),
    (deleted, ()),
    (other_targets, ([[1], [2]],)),
    (parameter, ((),)),
    (matching, ("go",)),
    (matching, ("stop",)),
    (in_try, ("a",)),
    (in_try, ("b",)),
    (drive, ()),
    (lambdas_and_defaults, ()),
]:
    report(f, *args)
    total.append(f.__name__)

print(asyncio.run(coroutine(3)))
print(len(total), seen, table["k"] is total)


def set_key_in_set():
    # `in` looks an unhashable set up as a frozenset, `PySet_Contains` does not.
    s = {frozenset({1}), 2}
    s.add(3)
    key = {1}
    print(key in s, {9} in s, key not in s, {1} in {frozenset({1})})
    try:
        print([1] in s)
    except TypeError as e:
        print(e)


def str_equal(x):
    # `Bound<PyString>` has its own `eq`.
    return str(x) == x, str(x) != x, repr(x) == x


set_key_in_set()
print(str_equal("a"), str_equal(1))
missing_key()
