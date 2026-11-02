# A direct call whose target is replaced from another module (here
# `unittest.mock`, patching by name). CPython calls the replacement;
# transpiled code still calls `f` and can not take `f`'s defaults from the
# replacement (design §12).
# steelix-test: expect-error TypeError: f() was replaced by <function g at ADDRESS>, but transpiled code calls the original directly and can not take the default of its parameter 'b' from the replacement
from unittest import mock


def f(a, b=[]):
    return a, b


def g(x):
    return x


print(f(1))
mock.patch("__main__.f", g).start()
print(f(1))
