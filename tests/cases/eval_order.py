"""Side effects must happen in Python's evaluation order."""


def trace(label, value):
    print("eval", label)
    return value


def make_box():
    import types
    box = types.SimpleNamespace()
    box.x = 10
    return box


def subscripts():
    data = [0, 0, 0]
    data[trace("index", 1)] = trace("value", 5)
    trace("target", data)[trace("index", 2)] = trace("value", 7)
    print(data)
    print(trace("container", data)[trace("index", 0)])


def attributes():
    box = make_box()
    trace("object", box).x = trace("value", 20)
    print(box.x)
    trace("object", box).x += trace("increment", 1)
    print(box.x)


def augmented():
    data = [10, 20]
    trace("target", data)[trace("index", 0)] += trace("value", 5)
    print(data)
    total = trace("start", 1)
    total += trace("step", 2)
    print(total)
    items = [1]
    alias = items
    items += [2]
    print(alias, items is alias)


def membership():
    print(trace("item", 2) in trace("container", [1, 2]))
    print(trace("item", 5) not in trace("container", [1, 2]))
    if trace("needle", "b") in trace("haystack", "abc"):
        print("contained")


def comparisons():
    print(trace("a", 1) < trace("b", 2) < trace("c", 3))
    print(trace("a", 3) < trace("b", 2) < trace("c", 1))
    if trace("low", 0) <= trace("mid", 5) <= trace("high", 10):
        print("in range")
    print(trace("x", 1) == trace("y", 1.0))


def calls():
    s = "a,b"
    print(trace("receiver", s).split(trace("separator", ",")))
    print(len(trace("arg", [1, 2])), max(trace("first", 3), trace("second", 4)))
    print([trace("e1", 1), trace("e2", 2)], {trace("k", "key"): trace("v", "value")})
    print(f"{trace('f1', 1)}-{trace('f2', 2):>{trace('width', 3)}}")


def boolean():
    print(trace("left", 0) or trace("right", "fallback"))
    print(trace("left", 1) and trace("right", "second"))
    print(trace("cond", True) and trace("unused", 0) or trace("default", 9))
    print(trace("then", "yes") if trace("test", 1) else trace("else", "no"))


def unpacking():
    data = [0, 0]
    data[trace("i", 0)], data[trace("j", 1)] = trace("pair", (1, 2))
    print(data)


def tricky():
    data = {"k": [0, 0]}
    print(trace("obj", "a-b").split(trace("sep", "-") in trace("seps", "-+")))
    print([trace("first", 1), trace("x", 1) in trace("xs", [1])])
    data[trace("key", "k")] = trace("left", 0) or trace("right", [5, 6])
    data[trace("outer", "k")][trace("inner", 0)] = trace("value", 7)
    print(data)
    result = trace("a", 1) < trace("b", 2) < trace("c", 3)
    print(result, trace("then", 1) if trace("x", 2) in trace("xs", [2]) else trace("else", 0))
    print(f"{trace('p1', 1)}{trace('needle', 2) in trace('hay', [2])}")
    data[trace("key", "k")][trace("index", 1)] += trace("x", 1) in trace("xs", [1])
    print(data)
    n = 0
    while trace("n", n) in trace("range", [0, 1, 2]):
        n = n + 1
    if trace("x", 0) in trace("xs", []):
        print("first")
    elif trace("y", 1) in trace("ys", [1]):
        print("second")
    assert trace("x", 1) in trace("xs", [1]), trace("message", "never evaluated")
    del data[trace("del key", "k")][trace("del index", 0)], data[trace("again", "k")]
    print(data, n)
    for item in trace("iterable", [1, 2]):
        print("item", item)


def bump():
    global counter
    counter = 100
    return 1


subscripts()
attributes()
augmented()
membership()
comparisons()
calls()
boolean()
unpacking()
tricky()
counter = 5
counter += bump()
print(counter)
counter = [0]
counter[trace("module index", 0)] += trace("module value", 3)
print(counter)
