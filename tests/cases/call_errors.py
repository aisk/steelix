"""Calls that raise TypeError get CPython's messages, with Python parameter names."""

import threading


def report(args):
    print(type(args.exc_value).__name__ + ":", args.exc_value)


threading.excepthook = report


def attempt(f, *args, **kwargs):
    # A thread reports the exception through `threading.excepthook`.
    worker = threading.Thread(target=f, args=args, kwargs=kwargs)
    worker.start()
    worker.join()


def f(a, b=1):
    return a


def g(x, /, y, *, z):
    return x


def h(*, a, b, c):
    return a


def k(globals, type, self):
    return globals


def no_args():
    return 0


def m(a, b, c, d):
    return a


attempt(f)
attempt(f, 1, 2, 3)
attempt(f, 1, a=2)
attempt(f, 1, c=2)
attempt(f, 1, bb=2)
attempt(g, x=1, y=2, z=3)
attempt(g, 1, 2, 3, z=4)
attempt(g, 1, 2)
attempt(h)
attempt(m)
attempt(m, 1, 2)
attempt(k)
attempt(k, 1, 2, 3, glob=4)
attempt(no_args, 1)
attempt(no_args, 1, 2, x=1)
attempt(print, *[1], **{"sep": "-"})
k(1, 2, self=3, type=4)
