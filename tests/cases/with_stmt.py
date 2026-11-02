# `with`: the protocol, suppression, targets, and jumps through `with`.
import os
import sys
import tempfile


class Manager:
    def __init__(self, name, suppress=False, fail_enter=False, fail_exit=False):
        self.name = name
        self.suppress = suppress
        self.fail_enter = fail_enter
        self.fail_exit = fail_exit

    def __enter__(self):
        print("enter", self.name)
        if self.fail_enter:
            raise RuntimeError("enter failed: " + self.name)
        return (self.name, len(self.name))

    def __exit__(self, exc_type, exc, tb):
        print("exit", self.name, exc_type.__name__ if exc_type else None, repr(exc),
              tb is not None, "handled:", repr(sys.exception()))
        if self.fail_exit:
            raise KeyError("exit failed: " + self.name)
        return self.suppress


class NoExit:
    def __enter__(self):
        return self


class Async:
    async def __aenter__(self):
        return self

    async def __aexit__(self, *args):
        return False


class InstanceMethods:
    pass


def write_and_read(path):
    with open(path, "w") as f:
        f.write("hello\n")
    with open(path) as f:
        return f.read()


def target_kinds():
    with Manager("abc") as (name, size):
        print("unpacked", name, size)
    holder = {}
    with Manager("d") as holder["value"]:
        pass
    print("stored", holder)
    with Manager("e"):
        print("no target")


def multiple():
    with Manager("outer") as a, Manager("inner") as b:
        print("both", a, b)


def suppressing():
    with Manager("quiet", suppress=True):
        raise ValueError("suppressed")
    print("after suppressed")


def not_suppressing():
    try:
        with Manager("loud"):
            raise ValueError("propagates")
    except ValueError as e:
        print("caught", e)


def enter_fails():
    try:
        with Manager("first"), Manager("second", fail_enter=True):
            print("not reached")
    except RuntimeError as e:
        print("caught", e)


def exit_fails():
    try:
        with Manager("ok", fail_exit=True):
            raise ValueError("original")
    except KeyError as e:
        print("caught", repr(e), "context", repr(e.__context__))


def jumps(items):
    found = None
    for item in items:
        with Manager(item):
            if item == "skip":
                continue
            if item == "stop":
                break
            if item == "ret":
                return "returned " + item
            found = item
    return found


def returns_value():
    with Manager("value") as v:
        return v


def bad_managers():
    for manager in [NoExit(), Async(), 42]:
        try:
            with manager:
                pass
        except TypeError as e:
            print("TypeError:", e)


def lookup_on_type():
    m = InstanceMethods()
    m.__enter__ = lambda: None
    m.__exit__ = lambda *a: None
    try:
        with m:
            pass
    except TypeError as e:
        print("TypeError:", e)


def suppress_in_loop():
    count = 0
    for i in range(3):
        with Manager(str(i), suppress=True):
            if i == 1:
                raise ValueError(i)
            count += 1
    return count


path = os.path.join(tempfile.mkdtemp(), "data.txt")
print(write_and_read(path))
target_kinds()
multiple()
suppressing()
not_suppressing()
enter_fails()
exit_fails()
print(jumps(["a", "skip", "b", "stop", "c"]))
print(jumps(["a", "ret"]))
print(returns_value())
bad_managers()
lookup_on_type()
print(suppress_in_loop())
with Manager("module") as pair:
    print("module level", pair)
with Manager("module quiet", suppress=True):
    raise ValueError("module")
print("pair" in globals(), pair)
