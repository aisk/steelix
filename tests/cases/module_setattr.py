# Rebinding a function through the module object or `exec` is seen by later
# calls: such modules call their functions through globals (design §7).
import sys
import types


def plain(x):
    return x * 2


def other(x):
    return x + 1


print(plain(21), other(1))
setattr(sys.modules[__name__], "plain", lambda x: -x)
print(plain(21))
me = sys.modules[__name__]
me.other = plain
print(other(1))
exec("def plain(x):\n    return x * 10")
print(plain(21), other(1))
# `vars(obj)` reads another namespace and does not count.
print(vars(types.SimpleNamespace(a=1)))
