"""CPython releases the locals when the function returns in reverse order of
`co_varnames` (parameters first, then the other locals by first use), Rust
in reverse order of declaration. The generated Rust declares them by first
use, which mostly gives the same order. A declaration merged with an `if`
chain past another one (`crossing`) does not: `KNOWN_DIFFERENCES` in
tests/test_e2e.py has it (design §12)."""


class T:
    def __init__(self, name):
        self.name = name

    def __del__(self):
        print("del", self.name)


def straight():
    b = T("b")
    a = T("a")
    c = T("c")
    print("straight", a.name, b.name, c.name)


def reassigned(x):
    y = T("y")
    x = T("x")
    print("reassigned", x.name, y.name)


def branches(n):
    # `a` and `c` are assigned on every path of an `if` chain.
    print("start")
    if n > 1:
        a = T("a-big")
    elif n == 1:
        a = T("a-one")
    else:
        return None
    b = T("b")
    if n > 2:
        c = T("c-" + a.name)
    else:
        print("no c yet")
        c = T("c-small")
    print("end", a.name, b.name, c.name)


def in_loop(n):
    first = T("first")
    for i in range(n):
        last = T(f"last-{i}")
    after = T("after")
    print("loop", first.name, last.name, after.name)


def without_else(n):
    head = T("head")
    if n:
        maybe = T("maybe")
    tail = T("tail")
    if n:
        print("without else", head.name, maybe.name, tail.name)


def in_try(text):
    try:
        number = int(text)
    except ValueError:
        value = T("bad " + text)
    else:
        value = T("good " + str(number))
    extra = T("extra")
    print("try", value.name, extra.name)


def crossing(n):
    # `b` is declared before the chain that gives `a` its value.
    if n:
        a = T("crossing a")
    else:
        b = T("crossing b")
        a = T("crossing a2")
    print("crossing", a.name)


straight()
print("--")
crossing(0)
print("--")
reassigned(None)
print("--")
for n in (0, 1, 3):
    branches(n)
    print("--")
in_loop(2)
print("--")
without_else(1)
print("--")
without_else(0)
print("--")
in_try("4")
print("--")
in_try("x")
