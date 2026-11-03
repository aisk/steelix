"""`async with` and `async for` (design §8): `frame.async_with`,
`rt::aiter` and `rt::anext`, with CPython's messages."""

import asyncio


class Resource:
    def __init__(self, name, suppress=False):
        self.name = name
        self.suppress = suppress

    async def __aenter__(self):
        print("enter", self.name)
        await asyncio.sleep(0)
        return self.name.upper()

    async def __aexit__(self, typ, exc, tb):
        await asyncio.sleep(0)
        print("exit", self.name, typ.__name__ if typ else None)
        return self.suppress


class Countdown:
    def __init__(self, n):
        self.n = n

    def __aiter__(self):
        return self

    async def __anext__(self):
        if self.n <= 0:
            raise StopAsyncIteration
        await asyncio.sleep(0)
        self.n -= 1
        return self.n


class SyncStop:
    def __aiter__(self):
        return self

    def __anext__(self):
        raise StopAsyncIteration


class NoExit:
    async def __aenter__(self):
        return self


class SyncOnly:
    def __enter__(self):
        return self

    def __exit__(self, *args):
        return False


class BadEnter:
    def __aenter__(self):
        return 1

    async def __aexit__(self, *args):
        return False


class BadExit:
    async def __aenter__(self):
        return self

    def __aexit__(self, *args):
        return 2


class BadNext:
    def __aiter__(self):
        return self

    def __anext__(self):
        return 3


class BadIter:
    def __aiter__(self):
        return 4


class NoIter:
    pass


async def with_flow(items):
    found = []
    for item in items:
        async with Resource(item) as value:
            if item == "skip":
                continue
            if item == "stop":
                break
            if item == "return":
                return found
            found.append(value)
    return found


async def for_flow(n):
    seen = []
    async for i in Countdown(n):
        if i == 3:
            continue
        if i == 1:
            break
        seen.append(i)
    else:
        seen.append("no break")
    async for i in Countdown(2):
        seen.append(i)
    else:
        seen.append("else")
    async for _ in SyncStop():
        seen.append("never")
    return seen


async def nested():
    async with Resource("a") as a, Resource("b") as b:
        print("body", a, b)
    async with Resource("c", suppress=True):
        raise KeyError("suppressed")
    try:
        async with Resource("d"):
            raise KeyError("raised")
    except KeyError as e:
        print("propagated", repr(e))
    async for i in Countdown(2):
        async with Resource(f"loop {i}") as value:
            print(value)


async def found_in(source, wanted):
    async for item in source:
        if item == wanted:
            return True
    return False


async def unpacking():
    pairs = []
    async for a, b in Pairs():
        pairs.append(a + b)
    return pairs


class Pairs:
    def __init__(self):
        self.items = [(1, 2), (3, 4)]

    def __aiter__(self):
        return self

    async def __anext__(self):
        if not self.items:
            raise StopAsyncIteration
        return self.items.pop(0)


async def errors():
    for cls in (NoExit, SyncOnly, BadEnter, BadExit, int):
        try:
            async with cls():
                print("body of", cls.__name__)
        except TypeError as e:
            print(cls.__name__, e, repr(e.__cause__), repr(e.__context__))
    for cls in (BadNext, BadIter, NoIter):
        try:
            async for x in cls():
                print("item", x)
        except TypeError as e:
            print(cls.__name__, e, repr(e.__cause__), repr(e.__context__))
    try:
        with Resource("sync"):
            pass
    except TypeError as e:
        print(e)


async def library():
    lock = asyncio.Lock()
    async with lock:
        print("locked", lock.locked())
    print("unlocked", lock.locked())
    try:
        async with asyncio.timeout(0.01):
            await asyncio.sleep(1)
    except TimeoutError:
        print("timed out")
    queue = asyncio.Queue()
    for i in range(3):
        queue.put_nowait(i)

    async def drain():
        while not queue.empty():
            yield_value = await queue.get()
            print("got", yield_value)

    await drain()


async def main():
    print(await with_flow(["x", "skip", "y", "stop", "z"]))
    print(await with_flow(["x", "return"]))
    print(await for_flow(5))
    await nested()
    print(await found_in(Countdown(5), 2), await found_in(Countdown(2), 7))
    print(await unpacking())
    await errors()
    await library()


asyncio.run(main())
