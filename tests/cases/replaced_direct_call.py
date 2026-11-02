# A direct call whose target is replaced through the module object.
# CPython calls the replacement; transpiled code still calls `f` and can not
# take `f`'s defaults from the replacement (design §12).
# steelix-test: expect-error TypeError: f() was replaced by <function g at ADDRESS>, but transpiled code calls the original directly and can not take the default of its parameter 'b' from the replacement
import sys


def f(a, b=[]):
    return a, b


def g(x):
    return x


print(f(1))
me = sys.modules[__name__]
me.f = g
print(f(1))
