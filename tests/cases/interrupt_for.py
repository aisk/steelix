# Ctrl-C stops a native `for` loop over an iterator implemented in C, which
# does not check for signals itself.
# steelix-test: interrupt after the first line of output
import itertools


def spin():
    total = 0
    for i in itertools.count():
        total += i
    return total


print("ready", flush=True)
spin()
