"""Functions that never return a value: their Rust fns return `PyResult<()>`,
and Python still sees `None` wherever the value is used."""

import functools

log = []


def note(label):
    log.append(label)
    print("note", label)


def proc(x):
    print("proc", x)


def early(x):
    if x < 0:
        print("negative")
        return
    if x == 0:
        return None
    print("positive", x)


def explicit_none():
    return None


def empty():
    pass


def only_docstring():
    """Does nothing."""


def mixed(x):
    if x:
        return x
    return


def tail_call(x):
    return proc(x)


def always_raises(message):
    raise ValueError(message)


def loop_return(items, stop):
    for item in items:
        if item == stop:
            return
        print("item", item)
    else:
        print("no stop")


def while_return(n):
    while True:
        n -= 1
        if n < 0:
            return
        print("n", n)


def match_return(value):
    match value:
        case 0:
            return
        case [a, b]:
            print("pair", a, b)
            return None
        case _:
            print("other", value)


def with_defaults(a, b=2, *rest, c="c", items=[], **options):
    print("with_defaults", a, b, rest, c, items, options)


def countdown(n):
    if n == 0:
        print("liftoff")
        return
    print(n)
    countdown(n - 1)


def value(label):
    print("value", label)
    return label


print("--- procedures")
proc(1)
early(-1)
early(0)
early(1)
empty()
only_docstring()
print(proc(2), early(0), explicit_none(), empty(), only_docstring())
print(mixed(3), mixed(0), tail_call(4))
loop_return([1, 2, 3], 2)
loop_return([1], 2)
while_return(2)
match_return(0)
match_return([1, 2])
match_return("x")
countdown(3)

print("--- uses of the value")
x = proc("assign")
print(x)
a = b = proc("chained")
print(a, b)
total = 1
total += 0 if proc("augmented") is None else 1
print(total)
print(proc("argument"))
print(value("before"), proc("middle"), value("after"))
print((value(1), proc("tuple"), value(2)))
print([value(1), proc("list"), value(2)])
print({value("k"): proc("dict value"), proc("dict key"): value("v")})
print({proc("set")})
print([*[value(0)], proc("starred display")])
print(proc("or") or "fallback")
print(proc("and") and "unused")
print(value(0) or proc("or rhs"))
print(value(1) and proc("and rhs"))
print(proc("is") is None, proc("is not") is not None)
print(proc("eq") == None, None != proc("ne"))  # noqa: E711
print(value(1) < value(2) and proc("chain") is None)
if proc("if"):
    print("truthy")
else:
    print("falsy")
if not proc("not"):
    print("not")
count = 0
while not proc("while") and count < 2:
    count += 1
while proc("while false"):
    print("never")
print(f"{proc('fstring')}")
print(f"[{proc('spec')!s:>6}] [{proc('repr')!r}] {value('x')}{proc('y')}{value('z')}")
print("yes" if proc("ifexp test") else "no")
print(proc("ifexp body") if value(1) else proc("unused"))
print(proc("unused") if value(0) else proc("ifexp orelse"))
assert proc("assert") is None
assert not proc("assert test"), proc("unused message")
print([proc(i) for i in range(2)])
print({i: proc(i) for i in range(2)})
print([i for i in range(3) if not proc(i)])
print(list(proc(i) for i in range(2)))
print(str(proc("str")), repr(proc("repr")), bool(proc("bool")), type(proc("type")))
print(value([1, 2])[proc("subscript") is None])
print(len([proc("len")]))
print(proc("attribute").__class__.__name__)
try:
    proc("call the value")()
except TypeError as e:
    print("TypeError", e)
try:
    proc("add") + value(1)
except TypeError as e:
    print("TypeError", e)
try:
    value(1) + proc("radd")
except TypeError as e:
    print("TypeError", e)
print(value("lhs") in [proc("in")], proc("not in") not in value([None]))
(y := proc("walrus"))
print(y)
p, q = proc("unpack 1"), proc("unpack 2")
print(p, q)
d = {}
d[value("key")] = proc("subscript store")
d[proc("key call")] = value("stored")
print(d)
del d[proc("del key")]
print(d)
for item in [proc("for iterable")]:
    print(item)
print(lambda_result := (lambda: proc("lambda"))())
print((lambda v=proc("lambda default"): v)())
print(proc(proc("nested")))
print(proc(value("inner")), proc(early(1)))
note(proc("statement argument"))

print("--- calls")
with_defaults(1)
with_defaults(1, c="C", b=value("b"))
with_defaults(value(1), value(2), value(3), value(4), c=value("c"), z=value("z"))
print(with_defaults(a=proc("keyword argument")))
args = (1, 2, 3)
options = {"c": "star", "k": "v"}
with_defaults(*args, **options)
print(with_defaults(*args))
with_defaults.__defaults__ = (20,)
with_defaults(10)
with_defaults(1)
print(with_defaults.__defaults__, with_defaults.__kwdefaults__)
print(proc.__name__, proc(0) is None, proc.__call__(1))
try:
    proc()
except TypeError as e:
    print("TypeError", e)
try:
    print(always_raises("caught"))
except ValueError as e:
    print("ValueError", e)


try:
    (stored := always_raises("walrus in try"))
except ValueError as e:
    print("ValueError", e)
try:
    (stored := proc("walrus in try"))
except ValueError as e:
    print("unreachable")
print(stored)

print("--- inside functions")


def uses():
    proc("statement")
    r = proc("local")
    print(r)
    r = [proc("a"), value("b"), proc("c")]
    print(r)
    if proc("condition") is None:
        print("is None")
    return early(5)


print(uses())


def returns_unit_call(flag):
    if flag:
        return proc("returned")
    return early(-1)


print(returns_unit_call(True), returns_unit_call(False))


def unit_calling_units(x):
    proc(x)
    early(x)
    empty()


print(unit_calling_units(7))


def order(x):
    return [value(1), proc(x), value(2)], value(3) if proc("cond") else value(4)


print(order("order"))

print("--- try, finally and with")


class Manager:
    def __init__(self, name, suppress=False):
        self.name = name
        self.suppress = suppress

    def __enter__(self):
        print("enter", self.name)
        return self

    def __exit__(self, kind, exc, tb):
        print("exit", self.name, kind.__name__ if kind else None)
        return self.suppress


def try_finally(x):
    try:
        if x == 0:
            return
        if x == 1:
            return None
        print("try", x)
    finally:
        print("finally", x)
    print("after", x)


def try_except(x):
    try:
        if x == 0:
            return
        always_raises("in try")
    except ValueError as e:
        print("caught", e)
        if x == 1:
            return
    else:
        print("else")
    print("end", x)


def try_always_returns(x):
    try:
        print("body", x)
        return
    finally:
        print("cleanup", x)


def try_except_returns(x):
    try:
        print("body", int(x))
        return
    except ValueError:
        print("not a number", x)


def try_return_none(x):
    try:
        return None
    except ValueError:
        print("unreachable", x)


def try_call(x):
    try:
        proc(x)
    except ValueError:
        print("unreachable")
    try:
        always_raises(x)
    except ValueError as e:
        print("caught", e)
    try:
        always_raises(value(x))
    except ValueError as e:
        print("caught", e)
    try:
        r = proc(x)
    except ValueError:
        r = "unreachable"
    try:
        return proc(r)
    except ValueError:
        print("unreachable")


def try_call_value(x):
    try:
        return always_raises(x)
    except ValueError as e:
        return str(e)


def with_return(x):
    with Manager("a"):
        if x == 0:
            return
        print("with", x)
    print("after with", x)


def with_always_returns():
    with Manager("b"), Manager("c"):
        return


def with_suppressed():
    with Manager("d", suppress=True):
        always_raises("suppressed")
    print("after suppressed")


def with_loop(items):
    for item in items:
        with Manager(item):
            if item == "skip":
                continue
            if item == "stop":
                break
            if item == "return":
                return
            print("body", item)
    print("loop done")


def nested_regions(x):
    try:
        with Manager("outer"):
            try:
                if x == 0:
                    return
                if x == 1:
                    always_raises("inner")
            finally:
                print("inner finally")
            print("in with")
    except ValueError as e:
        print("caught", e)
        return
    finally:
        print("outer finally")
    print("end", x)


def finally_returns():
    try:
        always_raises("dropped")
    finally:
        return


def loop_try(items):
    for item in items:
        try:
            if item == 1:
                continue
            if item == 2:
                break
            if item == 3:
                return
            print("item", item)
        finally:
            print("finally", item)
    print("done")


for n in range(3):
    print(try_finally(n))
    print(try_except(n))
    print(nested_regions(n))
print(try_always_returns(1))
print(try_except_returns("1"), try_except_returns("x"), try_return_none(1))
print(try_call("t"))
print(try_call_value("message"))
print(with_return(0), with_return(1))
print(with_always_returns())
print(with_suppressed())
print(with_loop(["one", "skip", "two", "stop", "three"]))
print(with_loop(["one", "return", "two"]))
print(finally_returns())
print(loop_try([0, 1, 2, 4]))
print(loop_try([0, 3, 4]))

print("--- nested functions and closures")


def counter():
    count = 0

    def bump(step=1):
        nonlocal count
        count += step

    def reset():
        nonlocal count
        if count == 0:
            return
        count = 0

    def show():
        print("count", count)

    print(bump(), bump(5))
    show()
    print(reset(), reset())
    show()
    return bump, show


bump, show = counter()
print(bump(2), show())
print(bump.__name__, bump.__qualname__, bump.__defaults__)


def recursive_nested(n):
    def down(k):
        if k == 0:
            return
        print("down", k)
        down(k - 1)

    return down(n)


print(recursive_nested(2))
call = lambda x: proc(x)  # noqa: E731
print(call("from lambda"))
print((lambda: [proc("in list"), early(0)])())

print("--- methods")


class Widget:
    instances = 0

    def __init__(self, name):
        self.name = name
        self._size = 0
        Widget.register()

    def rename(self, name):
        if name == self.name:
            return
        self.name = name

    @staticmethod
    def describe(text):
        print("describe", text)

    @classmethod
    def register(cls):
        cls.instances += 1

    @property
    def size(self):
        return self._size

    @size.setter
    def size(self, value):
        if value < 0:
            return
        self._size = value

    @property
    def nothing(self):
        print("nothing of", self.name)

    def __setitem__(self, key, value):
        print("setitem", key, value)

    def __call__(self):
        print("called", self.name)

    def __del__(self):
        print("del", self.name)


w = Widget("w")
print(w.rename("w"), w.rename("v"), w.name)
print(Widget.describe("static"), w.describe("instance"), Widget.register(), Widget.instances)
w.size = 3
w.size = -1
print(w.size, w.nothing)
w["k"] = "v"
print(w(), Widget.__init__(w, "again"), w.name)
print(Widget.rename.__qualname__, type(Widget.rename).__name__)


class BadInit:
    def __init__(self, flag):
        if flag:
            return None
        return


print(type(BadInit(True)).__name__, type(BadInit(False)).__name__)
w = None
print("after del")

print("--- decorators and callbacks")


def traced(fn):
    @functools.wraps(fn)
    def wrapper(*args, **kwargs):
        result = fn(*args, **kwargs)
        print("traced", fn.__name__, args, kwargs, result)
        return result

    return wrapper


def announce(fn):
    print("announce", fn.__name__, fn(0))
    return fn


@traced
def decorated(x, y=1):
    print("decorated", x, y)


@announce
def announced(x):
    if x:
        return
    print("announced", x)


def empty_key(item):
    log.append(item)


@functools.cache
def cached(x):
    print("computing", x)


print(decorated(1), decorated(2, y=3))
print(announced(1), announced(0))
print(cached(1), cached(1), cached(2))
print(list(map(proc, [1, 2])))
print(sorted([3], key=early), min([1], key=empty_key))
print(list(filter(proc, [1, 2])))
print(functools.reduce(lambda acc, item: proc(item), [1, 2], 0))
print(functools.partial(with_defaults, 1, c="partial")())
print(log)

print("--- uncaught")


def fails_late(x):
    proc(x)
    if x:
        always_raises("uncaught in a unit function")
    print("unreachable")


def outer():
    r = fails_late(1)
    print(r)


outer()
