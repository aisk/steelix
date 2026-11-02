"""Unpacking assignment, also in for loops and with starred targets."""


def swap(a, b):
    a, b = b, a
    return a, b


def unpack(seq):
    first, second = seq
    print(first, second)
    head, *rest = [1, 2, 3, 4]
    *init, last = "abc"
    left, *middle, right = range(5)
    print(head, rest, init, last, left, middle, right)
    x, (y, z) = 1, [2, 3]
    print(x, y, z)
    [p, q] = {"p": 1, "q": 2}
    print(p, q)
    return first


def targets():
    box = [0, 0, 0]
    box[0], box[2] = "a", "c"
    a = b = [5]
    a.append(6)
    print(box, a, b, a is b)
    total = 0
    for i, (x, y) in enumerate([(1, 2), (3, 4)]):
        total = total + i * (x + y)
    print(total)
    for key, value in {"k": "v"}.items():
        print(key, value)
    print(key)


def empty_star():
    a, *b = [1]
    *c, d = [2]
    print(a, b, c, d)


print(swap(1, 2))
print(unpack((7, 8)))
targets()
empty_star()
m, n = 10, 20
m, n = n, m
print(m, n)
first, *others = "xyz"
print(first, others)
for k, v in [("one", 1), ("two", 2)]:
    print(k, v)
