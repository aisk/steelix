"""Native coroutines and generators (PoC, design §8): every function in the
first half is hand-written as native Rust in `src/coroutines.rs`, the way
steelix would generate it; the drivers in the second half stay Python."""

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


# --- kept as Python ---


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

try:
    echo().send(1)
except TypeError as e:
    print("TypeError:", e)

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
        print("warning", w.category.__name__, w.message, w.lineno)
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
