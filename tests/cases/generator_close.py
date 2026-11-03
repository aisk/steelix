"""close() and throw() of generators and coroutines delegating to another one
(design §8): `yield from g(x)` and `await f(x)` between native functions run
the delegate in place (`rt::delegate`), through a variable they create an
object; both must follow CPython's `_gen_throw`."""

import sys


def via(f):
    return f


def swallows():
    try:
        yield 1
    except GeneratorExit:
        print("  swallows: caught GeneratorExit")
        return "swallowed"
    finally:
        print("  swallows: finally")
    yield "swallows continues"


def outer():
    try:
        r = yield from swallows()
        print("  outer continues with", r)
        yield "outer-after"
    except GeneratorExit:
        print("  outer: caught GeneratorExit")
        raise
    finally:
        print("  outer: finally")


def outer_via():
    try:
        r = yield from via(swallows)()
        print("  outer continues with", r)
        yield "outer-after"
    except GeneratorExit:
        print("  outer: caught GeneratorExit")
        raise
    finally:
        print("  outer: finally")


def yields_again():
    try:
        yield 1
    except GeneratorExit:
        print("  yields_again: yields again")
        yield "again"
    finally:
        print("  yields_again: finally")


def outer_of_yields_again():
    try:
        raise KeyError("outer's")
    except KeyError:
        try:
            yield from yields_again()
        except RuntimeError as e:
            print("  outer caught", repr(e), "context", repr(e.__context__))
            raise
        finally:
            print("  outer: finally")


def outer_of_yields_again_via():
    try:
        raise KeyError("outer's")
    except KeyError:
        try:
            yield from via(yields_again)()
        except RuntimeError as e:
            print("  outer caught", repr(e), "context", repr(e.__context__))
            raise
        finally:
            print("  outer: finally")


def keeps_yielding():
    while True:
        try:
            yield "x"
        except GeneratorExit:
            print("  keeps_yielding: ignores GeneratorExit")


def outer_of_keeps_yielding():
    try:
        yield from keeps_yielding()
    except RuntimeError as e:
        print("  outer caught", e)
    yield "after"


def raises_in_finally():
    try:
        yield 1
    finally:
        raise ValueError("from the delegate's finally")


def outer_of_raises():
    try:
        raise KeyError("outer's")
    except KeyError:
        try:
            yield from raises_in_finally()
        except ValueError as e:
            print("  outer got", repr(e), "context", repr(e.__context__))
            raise


def leaf():
    try:
        yield "leaf"
    except GeneratorExit as e:
        print("  leaf: caught, context", repr(e.__context__))
        return "leaf-ret"


def middle():
    try:
        r = yield from leaf()
        print("  middle continues", r)
        yield "middle-after"
    except GeneratorExit:
        print("  middle: caught")
        return "middle-ret"


def top():
    try:
        raise KeyError("top's")
    except KeyError:
        r = yield from middle()
        print("  top continues", r)
        yield "top-after"


def countdown(n):
    try:
        if n == 0:
            yield "bottom"
            return 0
        r = yield from countdown(n - 1)
        print("  countdown continues", n, r)
        yield "after"
    except GeneratorExit:
        print("  countdown caught", n)
        return n


def catches_thrown():
    try:
        yield 1
    except ValueError as e:
        print("  delegate: context", repr(e.__context__))
        raise TypeError("from the delegate") from None


def outer_of_catches():
    try:
        raise KeyError("outer's")
    except KeyError:
        yield from catches_thrown()


class Sleep:
    def __await__(self):
        yield "sleep"


async def aswallows():
    try:
        await Sleep()
    except GeneratorExit:
        print("  aswallows: caught GeneratorExit")
        return "swallowed"
    return "normal"


async def aouter():
    r = await aswallows()
    print("  aouter continues with", r)
    await Sleep()
    return "aouter done"


async def aouter_via():
    r = await via(aswallows)()
    print("  aouter continues with", r)
    await Sleep()
    return "aouter done"


async def aignores():
    try:
        await Sleep()
    except GeneratorExit:
        await Sleep()


async def aouter_of_ignores():
    try:
        await aignores()
    except RuntimeError as e:
        print("  aouter caught", e)
        return "aouter-ret"


def run(make, how):
    print(make.__name__, how)
    g = make()
    print("  first:", g.send(None))
    try:
        if how == "close":
            print("  close ->", g.close())
        elif how == "throw GeneratorExit":
            print("  throw ->", g.throw(GeneratorExit))
        else:
            print("  throw ->", g.throw(ValueError("thrown")))
    except BaseException as e:
        print("  raised", repr(e), "context", repr(e.__context__))
    try:
        print("  next ->", g.send(None))
    except BaseException as e:
        print("  next raised", repr(e))


def unraisable(u):
    # CPython's message ends with the repr of the generator object, which a
    # delegate run in place does not have (design §12).
    print("  unraisable:", u.err_msg.split(" <")[0], repr(u.exc_value))


sys.unraisablehook = unraisable
for how in ["close", "throw GeneratorExit", "throw ValueError"]:
    for make in [
        outer,
        outer_via,
        outer_of_yields_again,
        outer_of_yields_again_via,
        outer_of_keeps_yielding,
        outer_of_raises,
        top,
        lambda: countdown(3),
        outer_of_catches,
        aouter,
        aouter_via,
        aouter_of_ignores,
    ]:
        run(make, how)
    try:
        raise LookupError("the caller's")
    except LookupError:
        run(top, how)
sys.unraisablehook = sys.__unraisablehook__


# throw() passes on its arguments to a delegate that is not a generator.
class Throwable:
    def __iter__(self):
        return self

    def __next__(self):
        return "next"

    def __await__(self):
        return self

    def throw(self, *args):
        print("  throw got", [a if isinstance(a, type) else repr(a) for a in args])
        raise StopIteration("after throw")


def delegates(it):
    return (yield from it)


async def awaits(it):
    return await it


for make in [delegates, awaits]:
    for args in [(ValueError,), (ValueError("instance"),), (ValueError, "message")]:
        g = make(Throwable())
        g.send(None)
        try:
            import warnings

            with warnings.catch_warnings():
                warnings.simplefilter("ignore", DeprecationWarning)
                g.throw(*args)
        except StopIteration as e:
            print(make.__name__, "stopped with", e.value)


# Their classes look like CPython's.
def gen():
    yield 1


async def coro():
    pass


g = gen()
c = coro()
w = c.__await__()
for obj in [g, c, w, (x for x in [])]:
    print(type(obj), type(obj).__name__, type(obj).__module__, obj.__doc__ is None)
    try:
        obj[0]
    except TypeError as e:
        print(e)
print(type(w).__doc__)
c.close()
