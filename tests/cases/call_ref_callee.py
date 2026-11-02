"""A call with `*`/`**` of a local that may be unbound: the callee is a
reference returned by the unbound-local check."""


def run(flag, f, args, kwargs):
    if flag:
        action = f
    action(*args, **kwargs)


run(True, print, (1,), {"sep": ", "})
