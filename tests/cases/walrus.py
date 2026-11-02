"""Assignment expressions (`:=`), in comprehensions too, where they bind
in the enclosing function or module (PEP 572)."""


def trace(label, value):
    print(label, value)
    return value


def in_condition(values):
    if (n := len(values)) > 2:
        return f"long ({n})"
    return f"short ({n})"


def in_while(values):
    out = []
    it = iter(values)
    while (item := next(it, None)) is not None:
        out.append(item)
    return out, item


def order():
    # The read of `y` before the assignment sees the old value.
    y = 1
    total = y + (y := 10) + y
    return total, y


def in_comprehension(values):
    found = None
    if any((found := v) > 10 for v in values):
        print("found", found)
    total = [last := v * 2 for v in values]
    return total, last, found


def filtered(values):
    return [y for x in values if (y := x * 2) > 2], y


def captured():
    def get():
        return value

    value = 0
    (value := 5)
    return get()


def maybe_unbound(flag):
    if flag:
        (result := "set")
    try:
        return result
    except UnboundLocalError as exc:
        return type(exc).__name__


counter = 0


def global_walrus():
    global counter
    return (counter := counter + 1), counter


[(min := v) for v in (3, 7)]
print(in_condition([1, 2, 3]), in_condition([]), in_while([1, 2]), order())
print(in_comprehension([1, 20, 3]), filtered([1, 2, 3]), captured())
print(maybe_unbound(True), maybe_unbound(False), global_walrus(), global_walrus())
print(min, (z := trace("z", 3)) + z, z)
del min
