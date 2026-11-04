"""Native coroutines and generators (design §8): the program the M13 PoC
wrote by hand in Rust, driven by asyncio, by hand and by the GC."""

import asyncio
import collections.abc
import gc
import inspect
import re
import sys
import traceback
import warnings


class Suspend:
    """Awaitable that hands its value to whoever drives the coroutine and
    returns what is sent back."""

    def __init__(self, value):
        self.value = value

    def __await__(self):
        return (yield self.value)


# --- native coroutines ---


async def add_later(a, b):
    await asyncio.sleep(0)
    return a + b


async def ticker(name, n):
    for i in range(n):
        print("tick", name, i)
        await asyncio.sleep(0)
    return name * 2


async def combine(x):
    direct = await add_later(x, 1)
    alias = add_later
    through_object = await alias(x, 2)
    from_python = await py_double(x)
    return direct, through_object, from_python


async def guarded(log):
    try:
        log.append("started")
        await asyncio.sleep(10)
        log.append("not reached")
    finally:
        log.append("finally")
        await asyncio.sleep(0)
        log.append("after await in finally: " + type(sys.exception()).__name__)


async def failing(x):
    await asyncio.sleep(0)
    raise ValueError("bad " + str(x))


async def outer_fail():
    await failing(1)


async def catcher():
    try:
        await failing(2)
    except ValueError as e:
        print("caught", e)
        await asyncio.sleep(0)
        print("still handling", repr(sys.exception()))
    print("after handler", repr(sys.exception()))


async def echo():
    a = await Suspend("first")
    print("echo got", a)
    b = await Suspend("second")
    return a + b


async def cleanup(tag):
    try:
        await Suspend(tag)
    finally:
        print("cleanup finally", tag)


async def stubborn():
    try:
        await Suspend(1)
    except GeneratorExit:
        print("ignoring GeneratorExit")
        await Suspend(2)


async def reenter(box):
    try:
        box[0].send(None)
    except ValueError as e:
        print("reenter:", e)
    return "reentered"


async def stop_inside():
    raise StopIteration(5)


async def await_int():
    await 1


async def compute(n):
    doubled = await Request("double", n)
    return await Request("add", doubled, 1)


async def handler_state():
    try:
        raise KeyError(1)
    except KeyError:
        print("in handler", repr(sys.exception()))
        await Suspend("in handler")
        print("after resume", repr(sys.exception()))
    print("after the handler", repr(sys.exception()))


async def nested_guard(log):
    await guarded(log)


async def countdown(n):
    if n == 0:
        return 0
    return n + await countdown(n - 1)


# --- native generators ---


def count_up(n):
    i = 0
    while i < n:
        yield i
        i += 1


def accumulate():
    total = 0
    while True:
        x = yield total
        if x is None:
            return total
        total += x


def resilient():
    while True:
        try:
            yield "ready"
        except KeyError as e:
            print("handled", repr(e))


def with_finally():
    try:
        yield 1
        yield 2
    finally:
        print("generator finally")


def delegate():
    result = yield from py_inner()
    print("delegate got", result)
    yield result


def in_with(manager):
    with manager as value:
        yield value
        yield value + 1


def pep479():
    yield 1
    next(iter([]))


def bad_finally():
    try:
        yield 1
    finally:
        raise KeyError("in finally")


def closing_value():
    try:
        yield 1
    except GeneratorExit:
        return "closed value"


def inner_items(n):
    try:
        for i in range(n):
            received = yield i
            if received is not None:
                print("inner_items received", received)
    finally:
        print("inner_items finally")
    return "inner_items done"


def outer_items():
    result = yield from inner_items(3)
    print("outer_items got", result)
    yield result


def walk(n):
    if n:
        yield from walk(n - 1)
    yield n


async def chain(n):
    if n == 0:
        return 0
    return 1 + await chain(n - 1)


# The same through generator and coroutine objects (not direct calls).
def walk_objects(n, f=None):
    if n:
        yield from (f or walk_objects)(n - 1, f)
    yield n


async def chain_objects(n, f=None):
    if n == 0:
        return 0
    return 1 + await (f or chain_objects)(n - 1, f)


def in_except():
    try:
        raise KeyError("in_except")
    except KeyError:
        yield "in except"


async def sleep_in_except():
    try:
        raise KeyError("sleep_in_except")
    except KeyError:
        await asyncio.sleep(10)


async def native_awaits(awaitable):
    return await awaitable


def owner_values(self):
    try:
        for value in self.values:
            yield value
    finally:
        print("owner_values finally", self.tag)


async def owner_run(self):
    try:
        await self.event.wait()
    finally:
        print("owner_run finally", self.tag)


async def recovered(tag):
    try:
        raise KeyError(tag)
    except KeyError:
        pass
    try:
        await Suspend(tag)
    finally:
        print("recovered finally", tag)


# --- CPython's own, mixed with the native ones ---


exec(
    """
async def py_awaits(awaitable):
    return await awaitable


async def py_double(x):
    await asyncio.sleep(0)
    return x * 2


def py_inner():
    try:
        received = yield "inner 1"
        print("inner received", received)
        try:
            yield "inner 2"
        except KeyError:
            print("inner caught KeyError")
        yield "inner 3"
    finally:
        print("inner finally")
    return "inner result"
"""
)


class Manager:
    def __enter__(self):
        print("enter")
        return 10

    def __exit__(self, typ, value, tb):
        print("exit", typ.__name__ if typ else None)
        return False


class Request:
    def __init__(self, *args):
        self.args = args

    def __await__(self):
        return (yield self)


def frames(e):
    return [(f.name, f.lineno) for f in traceback.extract_tb(e.__traceback__)]


def unraisable(u):
    message = re.sub(r" at 0x[0-9a-f]+", "", u.err_msg or "")
    print("unraisable:", u.exc_type.__name__, u.exc_value, "|", message)


sys.unraisablehook = unraisable


async def main():
    print("combine", await combine(10))
    print("gather", await asyncio.gather(ticker("a", 2), ticker("b", 3)))
    log = []
    task = asyncio.create_task(guarded(log))
    await asyncio.sleep(0)
    task.cancel()
    try:
        await task
    except asyncio.CancelledError:
        print("cancelled", log, task.cancelled())
    for make in (nested_guard, lambda log: py_awaits(guarded(log))):
        log = []
        task = asyncio.create_task(make(log))
        await asyncio.sleep(0)
        task.cancel()
        try:
            await task
        except asyncio.CancelledError as e:
            print("cancelled", log, [name for name, _ in frames(e)])
    try:
        await outer_fail()
    except ValueError as e:
        print("traceback", e, frames(e))
    await catcher()
    print("countdown", await countdown(30))


print("--- asyncio")
asyncio.run(main())

print("--- send, throw, close")
c = echo()
print(c.send(None))
print(c.send(1))
try:
    c.send(2)
except StopIteration as e:
    print("StopIteration", e.value, e.args)
try:
    c.send(None)
except RuntimeError as e:
    print("reuse:", e)
try:
    c.throw(ValueError("late"))
except RuntimeError as e:
    print("reuse throw:", e)
print("close after end", c.close())

c = echo()
c.send(None)
try:
    c.throw(KeyError("thrown"))
except KeyError as e:
    print("throw propagates", repr(e), frames(e)[1:])

c = cleanup("closed")
print(c.send(None))
print("close returns", c.close(), c.cr_suspended)

c = stubborn()
c.send(None)
try:
    c.close()
except RuntimeError as e:
    print(e)
print("still suspended", c.cr_suspended)
del c
print("after del stubborn")

box = []
c = reenter(box)
box.append(c)
try:
    c.send(None)
except StopIteration as e:
    print("reenter returned", e.value)
del box, c

try:
    stop_inside().send(None)
except RuntimeError as e:
    print(e, repr(e.__cause__))

try:
    await_int().send(None)
except TypeError as e:
    print("TypeError:", e)

with warnings.catch_warnings(record=True) as caught:
    warnings.simplefilter("always")
    try:
        echo().send(1)
    except TypeError as e:
        print("TypeError:", e)
    gc.collect()
    for w in caught:
        print("warning", w.category.__name__, w.message)

c = echo()
try:
    c.throw(KeyError("before start"))
except KeyError as e:
    print("thrown before start", repr(e), frames(e)[1:])
try:
    c.send(None)
except RuntimeError as e:
    print("after throw:", e)

c = handler_state()
print(c.send(None))
print("outside", repr(sys.exception()))
try:
    raise ValueError("caller")
except ValueError:
    try:
        c.send(None)
    except StopIteration:
        pass

print("--- finalization")
with warnings.catch_warnings(record=True) as caught:
    warnings.simplefilter("always")
    c = add_later(1, 2)
    del c
    gc.collect()
    c = add_later(1, 2)
    c.close()
    del c
    for w in caught:
        # The line is 0 for a warning without a Python frame (design §12).
        print("warning", w.category.__name__, w.message)
with warnings.catch_warnings():
    warnings.simplefilter("error")
    c = add_later(1, 2)
    del c
    print("warning as error reported")

c = cleanup("deleted")
c.send(None)
print("before del")
del c
print("after del")

cycle = [cleanup("in a cycle")]
cycle.append(cycle)
cycle[0].send(None)
del cycle
print("before collect")
gc.collect()
print("after collect")


class Holder:
    def __str__(self):
        return "holder"


holder = Holder()
holder.coroutine = cleanup(holder)
holder.coroutine.send(None)
del holder
gc.collect()
print("after collecting a cycle through the locals")

w = with_finally()
next(w)
print("before del generator")
del w
print("after del generator")
b = bad_finally()
next(b)
del b
print("after bad_finally")

print("--- introspection")
c = add_later(1, 2)
print("iscoroutinefunction", inspect.iscoroutinefunction(add_later))
print("isgeneratorfunction", inspect.isgeneratorfunction(count_up))
print("asyncio.iscoroutine", asyncio.iscoroutine(c))
print("Coroutine abc", isinstance(c, collections.abc.Coroutine))
print("inspect.iscoroutine", inspect.iscoroutine(c))
print("names", c.__name__, c.__qualname__, repr(c).split(" at ")[0])
c.close()
g = count_up(1)
print("Generator abc", isinstance(g, collections.abc.Generator))
print("names", g.__name__, g.__qualname__, repr(g).split(" at ")[0])

print("--- trampoline")


def trampoline(coro):
    value = None
    while True:
        try:
            request = coro.send(value)
        except StopIteration as e:
            return e.value
        op, *args = request.args
        print("request", op, args)
        value = args[0] * 2 if op == "double" else sum(args)


print("trampoline", trampoline(compute(20)))

print("--- generators")
print(list(count_up(4)))
for i in count_up(2):
    print("for", i)

acc = accumulate()
print(next(acc), acc.send(5), acc.send(7))
try:
    acc.send(None)
except StopIteration as e:
    print("accumulated", e.value)

r = resilient()
print(next(r))
print(r.throw(KeyError("k")))
r.close()
print("closed", r.gi_suspended)

w = with_finally()
print(next(w))
w.close()
print("closed with finally")

d = delegate()
print(next(d))
print(d.send("hello"))
print(d.throw(KeyError("x")))
print(next(d))
try:
    next(d)
except StopIteration:
    print("delegate done")
d = delegate()
next(d)
d.close()
print("delegate closed")

m = in_with(Manager())
print(next(m), next(m))
m.close()
m = in_with(Manager())
print(next(m))
try:
    m.throw(ValueError("in with"))
except ValueError as e:
    print("with saw", e)

v = closing_value()
next(v)
print("close returns", v.close())

p = pep479()
print(next(p))
try:
    next(p)
except RuntimeError as e:
    print(e, repr(e.__cause__))

print("--- direct await of a native generator (yield from)")
o = outer_items()
print(next(o), o.send("sent"), next(o), next(o))
try:
    next(o)
except StopIteration:
    print("outer_items done")
o = outer_items()
next(o)
o.close()
print("outer_items closed")
o = outer_items()
next(o)
try:
    o.throw(KeyError("into inner"))
except KeyError as e:
    print("thrown through", repr(e), [name for name, _ in frames(e)])

print("--- recursion depth")
# Too deep a chain raises `RecursionError`: CPython at the recursion limit,
# native code when the C stack runs out (design §12), so only the type is
# compared.
for n in (100, 100_000):
    for name, run in (
        ("walk", lambda: len(list(walk(n)))),
        ("chain", lambda: asyncio.run(chain(n))),
        ("countdown", lambda: asyncio.run(countdown(n))),
        ("walk_objects", lambda: len(list(walk_objects(n)))),
        ("chain_objects", lambda: asyncio.run(chain_objects(n))),
    ):
        try:
            print(name, n, run())
        except RecursionError as e:
            print(name, n, type(e).__name__)

print("--- context of thrown exceptions")
g = in_except()
print(next(g))
try:
    g.throw(ValueError("thrown"))
except ValueError as e:
    print("thrown context", repr(e.__context__))

# A body that handles nothing itself gives what is thrown in no context,
# whatever the caller handles.
g = walk(0)
next(g)
try:
    raise KeyError("the caller's")
except KeyError:
    try:
        g.throw(ValueError("thrown"))
    except ValueError as e:
        print("thrown context without a handler", repr(e.__context__))


# A body suspended in an `except` block that is dropped (here it ignores
# `close()`) leaves the exception handled where it is dropped alone.
def stubborn_in_except():
    try:
        raise KeyError("stubborn")
    except KeyError:
        while True:
            try:
                yield
            except GeneratorExit:
                pass


sys.unraisablehook = lambda u: print("unraisable:", repr(u.exc_value))
g = stubborn_in_except()
next(g)
try:
    raise ValueError("mine")
except ValueError:
    del g
    print("handled after the drop", repr(sys.exception()))
sys.unraisablehook = sys.__unraisablehook__


async def cancel_in_except():
    task = asyncio.create_task(sleep_in_except())
    await asyncio.sleep(0)
    task.cancel()
    try:
        await task
    except asyncio.CancelledError as e:
        chain = []
        while e is not None:
            chain.append(repr(e))
            e = e.__context__
        print("cancelled chain", chain)


asyncio.run(cancel_in_except())

print("--- awaited already")


async def awaited_already():
    for make in (lambda: add_later(1, 2), lambda: py_double(1)):
        c = make()
        task = asyncio.ensure_future(c)
        await asyncio.sleep(0)
        for waiter in (py_awaits, native_awaits):
            try:
                await waiter(c)
            except RuntimeError as e:
                print("awaited already:", e, c.cr_suspended)
        print("task result", await task)


asyncio.run(awaited_already())

print("--- cycles through self")


class Owner:
    values_of = owner_values
    run = owner_run

    def __init__(self, tag):
        self.tag = tag
        self.values = [1, 2]

    def __del__(self):
        print("del owner", self.tag)


owner = Owner("generator")
owner.generator = owner.values_of()
print(next(owner.generator))
del owner
gc.collect()
print("after collecting an owner and its generator")


def report(loop, context):
    print("exception handler:", context["message"])


async def owner_task():
    asyncio.get_running_loop().set_exception_handler(report)
    owner = Owner("task")
    owner.event = asyncio.Event()
    owner.task = asyncio.create_task(owner.run())
    await asyncio.sleep(0)
    del owner
    gc.collect()
    print("after collecting an owner and its task")


asyncio.run(owner_task())

print("--- at exit")
# The hook is a function of this module: it would keep the globals alive.
sys.unraisablehook = sys.__unraisablehook__


class Last:
    def __del__(self):
        print("del last")


never_started = count_up(3)
suspended_generator = with_finally()
next(suspended_generator)
suspended_coroutine = cleanup("at exit")
suspended_coroutine.send(None)
after_an_error = recovered("after an error")
after_an_error.send(None)
last = Last()
print("end of script")
