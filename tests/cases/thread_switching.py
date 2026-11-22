# Native loops let other threads take the GIL, as CPython's eval loop does
# after `sys.getswitchinterval()`: each loop below waits for a flag that
# another thread sets, and gives up after far more iterations than that
# takes, so a loop that keeps the GIL reports False.
import threading
import time

LIMIT = 20_000_000
done = False


def finish():
    global done
    # Starting the thread lets it run; wait until the loop has started.
    time.sleep(0.001)
    done = True


def race(name, spin):
    global done
    done = False
    worker = threading.Thread(target=finish)
    worker.start()
    result = spin()
    worker.join()
    print(name, result)


def spin_while():
    n = 0
    while not done and n < LIMIT:
        n += 1
    return done


def spin_for():
    for _ in range(LIMIT):
        if done:
            break
    return done


def spin_comprehension():
    seen = [i for i in range(LIMIT) if not done]
    return len(seen) < LIMIT


def spin_genexpr():
    # `any` is C: only the generator expression's resumptions run native code.
    return any(done for _ in range(LIMIT))


def flags():
    for _ in range(LIMIT):
        yield done


def spin_generator():
    return any(flags())


async def spin_async():
    n = 0
    while not done and n < LIMIT:
        n += 1
    return done


def run_coroutine():
    try:
        spin_async().send(None)
    except StopIteration as stop:
        return stop.value


def walk(depth):
    # Recursion without loops: only the calls can let the other thread in.
    if done or depth == 0:
        return 0
    return walk(depth - 1) + walk(depth - 1) + 1


def spin_recursion():
    walk(24)
    return done


race("while", spin_while)
race("for", spin_for)
race("comprehension", spin_comprehension)
race("generator expression", spin_genexpr)
race("generator", spin_generator)
race("coroutine", run_coroutine)
race("recursion", spin_recursion)
