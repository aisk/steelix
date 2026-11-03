"""Replacing the metadata of a function (or a generator expression) drops the
old value, whose `__del__` may read the same metadata again, from this thread
or from another one. The runtime must not hold its lock while that runs."""

import threading


class Nasty:
    def __init__(self, what):
        self.what = what

    def __del__(self):
        print("del", self.what, "reads", f.__name__, f.__qualname__, f.__defaults__)


class NastyName(str):
    def __del__(self):
        print("del name reads", f.__name__, g.__name__)


def f(a=None, *, b=None):
    """Original doc."""
    return a, b


print("start", f.__doc__)
f.__doc__ = Nasty("doc")
f.__doc__ = None
print("doc replaced")
f.__defaults__ = (Nasty("defaults"),)
f.__defaults__ = (1,)
print("defaults replaced", f())
f.__kwdefaults__ = {"b": Nasty("kwdefaults")}
f.__kwdefaults__ = None
print("kwdefaults replaced")
f.__module__ = Nasty("module")
f.__module__ = "elsewhere"
print("module replaced")
f.__annotations__ = {"a": Nasty("annotations")}
f.__annotations__ = {}
print("annotations replaced")
f.__annotations__ = {"a": Nasty("annotations before annotate")}
f.__annotate__ = None
print("annotate cleared", list(f.__annotations__))
f.__annotate__ = lambda format: {"a": int}
print("annotate replaced", f.__annotations__)
f.__annotations__ = {"a": Nasty("annotations before annotate")}
f.__annotate__ = None
f.__annotations__ = {}
f.__type_params__ = (Nasty("type params"),)
f.__type_params__ = ()
print("type params replaced")
g = (x for x in range(3))
f.__name__ = NastyName("renamed")
f.__name__ = "f"
print("name replaced")
f.__qualname__ = NastyName("requalified")
f.__qualname__ = "f"
print("qualname replaced")

g.__name__ = NastyName("genexpr name")
g.__name__ = "plain"
print("generator name replaced", g.__name__, list(g))


# Another thread reads the metadata while a `__del__` runs in this thread and
# waits for it.
class Slow:
    def __del__(self):
        started.set()
        done.wait(5)
        print("slow del done")


started = threading.Event()
done = threading.Event()


def reader():
    started.wait(5)
    print("reader sees", f.__qualname__, f.__doc__)
    done.set()


thread = threading.Thread(target=reader)
thread.start()
f.__doc__ = Slow()
f.__doc__ = "fresh"
thread.join(10)
print("end", thread.is_alive(), f.__doc__)
