"""`async def` lowered to native `async fn`s, run by asyncio (design §8)."""

import asyncio
import functools
import inspect


async def double(x):
    await asyncio.sleep(0)
    return x * 2


async def fib(n):
    """Recursive direct awaits: `Box::pin`."""
    if n < 2:
        return n
    return await fib(n - 1) + await fib(n - 2)


async def is_even(n):
    return True if n == 0 else await is_odd(n - 1)


async def is_odd(n):
    return False if n == 0 else await is_even(n - 1)


async def worker(name, delay, log):
    log.append(f"{name} start")
    await asyncio.sleep(delay)
    log.append(f"{name} end")
    return name


async def slow(log):
    try:
        log.append("slow start")
        await asyncio.sleep(10)
        log.append("slow end")
    finally:
        log.append("slow finally")


async def raises(message):
    await asyncio.sleep(0)
    raise ValueError(message)


async def handles():
    try:
        await raises("first")
    except ValueError as e:
        await asyncio.sleep(0)
        print("handled", e)
    try:
        value = await double(21)
    except KeyError:
        value = None
    else:
        print("else", value)
    for i in range(3):
        try:
            if i == 1:
                continue
            await asyncio.sleep(0)
            if i == 2:
                break
        finally:
            print("loop finally", i)
    while True:
        try:
            await raises("in loop")
        except ValueError:
            return value


def traced(fn):
    @functools.wraps(fn)
    async def wrapper(*args):
        print("call", fn.__name__, args)
        result = await fn(*args)
        print("result", result)
        return result

    return wrapper


@traced
async def add(a, b):
    await asyncio.sleep(0)
    return a + b


def make_accumulator():
    total = 0

    async def add_to(x):
        nonlocal total
        await asyncio.sleep(0)
        total += x
        return total

    return add_to


async def outer(n):
    async def inner(k):
        await asyncio.sleep(0)
        return n * k

    results = []
    for k in range(3):
        results.append(await inner(k))
    return results


class Base:
    def __init__(self, value):
        self._value = value

    async def get(self):
        await asyncio.sleep(0)
        return self._value

    @property
    def value(self):
        return self.get()

    @staticmethod
    async def static(x):
        return x + 1

    @classmethod
    async def make(cls, value):
        await asyncio.sleep(0)
        return cls(value)


class Derived(Base):
    async def get(self):
        base = await super().get()
        return base * 10


async def main():
    print(await double(4))
    print(await fib(10))
    print(await is_even(7), await is_odd(7))
    log = []
    results = await asyncio.gather(
        worker("a", 0.02, log), worker("b", 0.01, log), double(5)
    )
    print(results, log)

    log = []
    task = asyncio.create_task(slow(log))
    await asyncio.sleep(0)
    task.cancel()
    try:
        await task
    except asyncio.CancelledError:
        print("cancelled", log, task.cancelled())

    try:
        await asyncio.gather(raises("in gather"), double(1))
    except ValueError as e:
        print("gather raised", e)

    print("handles returned", await handles())
    print(await add(1, 2))
    acc = make_accumulator()
    print(await acc(1), await acc(2))
    print(await outer(3))

    d = await Derived.make(4)
    print(type(d).__name__, await d.get(), await d.value, await Base.static(1))

    # Through the function object: a coroutine object is created first.
    coro = double(8)
    print(type(coro).__name__, coro.__name__, coro.__qualname__, asyncio.iscoroutine(coro))
    print(await coro)
    alias = double
    print(await alias(9))
    print(inspect.iscoroutinefunction(double), inspect.iscoroutinefunction(Derived.get))
    async with asyncio.TaskGroup() as group:
        first = group.create_task(double(1))
        second = group.create_task(double(2))
    print(first.result(), second.result())


asyncio.run(main())
