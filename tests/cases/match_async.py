"""`match` in native coroutines and generators (design §8): `await` in the
subject, a guard and a case body, `yield` in a body, captures kept across
suspensions."""

import asyncio


async def fetch(value):
    await asyncio.sleep(0)
    return value


async def allowed(name):
    await asyncio.sleep(0)
    return name != "root"


async def handle(request):
    match await fetch(request):
        case {"user": name, "action": "login"} if await allowed(name):
            await asyncio.sleep(0)
            return f"welcome {name}"
        case {"user": name, "action": "login"}:
            return f"denied {name}"
        case {"action": action, **rest}:
            extra = await fetch(sorted(rest))
            return f"{action} with {extra}"
        case [first, *others]:
            total = first
            for other in others:
                total += await fetch(other)
            return f"sum {total}"
    return "unknown"


async def main():
    requests = [
        {"user": "ann", "action": "login"},
        {"user": "root", "action": "login"},
        {"action": "logout", "user": "ann", "at": 3},
        [1, 2, 3],
        "noise",
    ]
    for request in requests:
        print(await handle(request))
    # A capture assigned before an `await` in the guard keeps its value.
    match [5, 6]:
        case [a, b] if await fetch(a > b):
            print("no")
        case _:
            print("after the guard", a, b)


asyncio.run(main())


def tokens(text):
    """A generator: `yield` in the case bodies."""
    for word in text.split():
        match word:
            case "+" | "-" | "*" | "/" as op:
                yield ("op", op)
            case str() if word.isdigit():
                yield ("num", int(word))
            case str() if word.startswith("#"):
                return
            case _:
                yield ("name", word)
                yield ("len", len(word))


print(list(tokens("1 + x * 22 # rest is a comment")))


def lines(rows):
    """Captures used after a `yield` are kept in the generator's state."""
    for row in rows:
        match row:
            case (key, value):
                received = yield key
                print("sent", received, "for", key, value)
            case _:
                pass
    yield "done"


gen = lines([("a", 1), "skip", ("b", 2)])
print(next(gen))
print(gen.send("x"))
print(gen.send("y"))
