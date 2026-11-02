"""Default, keyword, keyword-only, positional-only, *args and **kwargs parameters."""


def greet(name, greeting="Hello", *, punct="!"):
    return greeting + ", " + name + punct


def span(start, stop=None, /, step=1):
    if stop is None:
        start, stop = 0, start
    return [start, stop, step]


def collect(first, *rest, sep="-", **options):
    return [first, rest, sep, options]


def append_to(item, bucket=[]):
    # The default list is created once and shared by every call.
    bucket.append(item)
    return bucket


def keyword_only(*, a, b=2):
    return a * 10 + b


def scale(x, factor=10 * 2):
    return x * factor


# Direct calls with omitted defaults, keywords in any order, *args and **kwargs.
print(greet("Bob"), greet("Al", greeting="Hi"), greet(punct="?", name="X"))
print(span(5), span(2, 8), span(2, 8, step=3))
print(collect(1), collect(1, 2, 3, sep="+", x=1, y=2))
print(append_to(1), append_to(2), append_to(3, []))
print(keyword_only(a=1), keyword_only(b=5, a=2))
print(scale(2))

# The same through the function objects.
call = greet
print(call("Bob"), call(name="Kim", punct="."))
print(collect.__defaults__, collect.__kwdefaults__, greet.__kwdefaults__)

# Defaults are read from the function object at call time.
greet.__defaults__ = ("Howdy",)
greet.__kwdefaults__ = {"punct": "?!"}
scale.__defaults__ = (3,)
print(greet("Sam"), call("Sam"), scale(2))
greet.__kwdefaults__ = None
print(greet.__kwdefaults__, keyword_only.__defaults__)
print(append_to.__defaults__)
