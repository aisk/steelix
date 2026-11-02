# `vars(obj)` reads the namespace of `obj`, not the module's, so the module
# still calls its functions directly (design §7).
import types


def double(x):
    return x * 2


point = types.SimpleNamespace(x=1, y=2)
print(vars(point), double(21))
