"""An exception raised through awaits, `yield from` and `async with`:
the traceback has CPython's frames and lines (design §8, §9)."""

import asyncio


def numbers(n):
    for i in range(n):
        if i == 2:
            raise LookupError(f"no {i}")
        yield i


def delegate(n):
    total = 0
    for value in numbers(n):
        total += value
    yield total
    yield from numbers(n)


class Guard:
    async def __aenter__(self):
        return self

    async def __aexit__(self, typ, exc, tb):
        print("exit with", typ.__name__)
        return False


async def leaf(n):
    await asyncio.sleep(0)
    values = []
    for value in delegate(n):
        values.append(value)
    return values


async def middle(n):
    async with Guard():
        try:
            return await leaf(n)
        finally:
            print("middle finally")


async def top():
    print(await leaf(2))
    return await middle(5)


asyncio.run(top())
