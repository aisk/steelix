"""Direct calls pass immutable literal defaults in place."""


def greet(name, greeting="Hello", *, punct="!", times=1, sep=None):
    text = greeting + ", " + name + punct
    if sep is None:
        sep = " "
    return sep.join([text] * times)


def scaled(x, factor=2.5, flag=True, raw=b"\x00", big=10**20):
    return [x * factor, flag, raw, big]


def remember(item, seen=[]):
    # A mutable default is still read from the function object.
    seen.append(item)
    return seen


print(greet("Bob"), greet("Al", times=2), greet("X", "Hi", sep="-", times=2))
print(scaled(2), scaled(1, 4))
print(remember(1), remember(2))
print(greet.__defaults__, greet.__kwdefaults__, scaled.__defaults__)
