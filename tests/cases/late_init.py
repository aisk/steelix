"""Locals that every branch of an `if` chain or a `try` assigns last are
declared with the value of the chain: `let x = if .. { a } else { b };`
(design §8)."""


class Tracked:
    def __init__(self, name):
        self.name = name
        print("new", name)

    def __del__(self):
        print("del", self.name)

    def __repr__(self):
        return f"Tracked({self.name})"


def sign(n):
    # A branch that returns needs no value.
    if n < 0:
        word = "negative"
    elif n == 0:
        return "zero"
    else:
        print("positive", n)
        word = "positive"
    return word


def nested(a, b):
    # A nested `if` that assigns in every branch is a value too.
    if a:
        if b:
            pair = "both"
        else:
            pair = "a only"
    elif b:
        pair = "b only"
    else:
        raise ValueError("neither")
    return pair


def flag(n):
    # A bool local.
    if n > 10:
        big = True
    else:
        big = n in (5, 10)
    return big


def counted(items):
    # Assigned again later: stays `mut`.
    if items:
        total = len(items)
    else:
        total = 0
    total += 1
    return total


def lifetimes(choose):
    # Objects are created and released when CPython does: the temporary in
    # the branch right away, the local when the function returns.
    if choose:
        Tracked("temporary")
        held = Tracked("then")
    else:
        held = Tracked("else")
    print("holding", held)
    return held.name


def parse(text):
    # `try` with `except` and `else` that both assign.
    try:
        print("parsing", text)
        number = int(text)
    except ValueError:
        print("not a number:", text)
        result = None
    else:
        result = number * 2
    return result


def partial(n):
    # Not on every path: keeps the declaration.
    if n > 0:
        label = "some"
    if n > 5:
        label = "many"
    try:
        return label
    except UnboundLocalError:
        return "unbound"


print(sign(-1), sign(0), sign(3))
print(nested(1, 1), nested(1, 0), nested(0, 1))
try:
    nested(0, 0)
except ValueError as err:
    print("ValueError:", err)
print(flag(11), flag(10), flag(1))
print(counted([1, 2]), counted([]))
print(lifetimes(True))
print(lifetimes(False))
print(parse("21"), parse("x"))
print(partial(0), partial(1), partial(9))
