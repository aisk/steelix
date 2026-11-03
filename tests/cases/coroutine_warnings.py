"""What follows from native coroutines and generators having no Python frame
(design §12): warnings they cause are attributed to `<sys>:0`, so the
default filters hide `DeprecationWarning`s and repeated warnings are shown
once, and an asyncio task's repr does not say where the coroutine is."""

import asyncio
import gc
import warnings


def gen():
    try:
        yield 1
    except ValueError:
        yield 2


async def coro():
    await asyncio.sleep(0)


def make_coroutines():
    first = coro()
    del first
    second = coro()
    del second
    gc.collect()


def throw_three_arguments():
    g = gen()
    next(g)
    return g.throw(ValueError, ValueError("v"), None)


# With the default filters, as a program runs.
with warnings.catch_warnings(record=True) as caught:
    print("throw returned", throw_three_arguments())
    make_coroutines()
for category in [DeprecationWarning, RuntimeWarning]:
    shown = [w for w in caught if w.category is category]
    print(category.__name__, "shown with the default filters:", len(shown))

# Everything shown, each time.
with warnings.catch_warnings(record=True) as caught:
    warnings.simplefilter("always")
    throw_three_arguments()
    make_coroutines()
print("warnings with 'always':", len(caught))


async def main():
    async def pending():
        await asyncio.sleep(10)

    task = asyncio.create_task(pending(), name="T")
    await asyncio.sleep(0)
    print("task repr says where:", "running at" in repr(task))
    task.cancel()
    try:
        await task
    except asyncio.CancelledError:
        print("cancelled")


asyncio.run(main())
