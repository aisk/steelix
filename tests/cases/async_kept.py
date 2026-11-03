"""What stays Python in coroutines and generators (design §8, §13), next
to native ones that call it."""

import asyncio
import types


async def ticks(n):
    # An asynchronous generator.
    for i in range(n):
        await asyncio.sleep(0)
        yield i


async def collect(n):
    # An asynchronous comprehension.
    return [i async for i in ticks(n)]


async def doubled(values):
    # `await` in a comprehension.
    return [await asyncio.sleep(0, v * 2) for v in values]


def lazy():
    # `yield` in a lambda.
    return lambda: (yield 1)


@types.coroutine
def legacy():
    # A generator that is a coroutine.
    value = yield from asyncio.sleep(0, "legacy").__await__()
    return value


async def handled(make_error):
    # `await` in an `except` clause.
    try:
        raise make_error()
    except await asyncio.sleep(0, KeyError):
        return "caught"


async def with_default(x=None):
    # `await` in the default of a nested function.
    def inner(y=await asyncio.sleep(0, 5)):
        return y

    return inner() if x is None else x


async def native(n):
    total = 0
    async for i in ticks(n):
        total += i
    return total, await collect(n), await doubled([1, 2])


def native_gen():
    yield from lazy()()
    yield "native"


async def main():
    print(await native(3))
    print(list(native_gen()))
    print(await legacy())
    print(await handled(KeyError), await with_default(), await with_default(1))


asyncio.run(main())
