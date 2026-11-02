# Ctrl-C in a native loop inside `try`: `finally` runs, then the
# `KeyboardInterrupt` traceback shows the native frames.
# steelix-test: interrupt after the first line of output


def spin():
    n = 0
    try:
        print("ready", flush=True)
        while True:
            n += 1
    finally:
        print("cleanup", n > 0)


spin()
