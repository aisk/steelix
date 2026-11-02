# Ctrl-C stops a native loop with `KeyboardInterrupt`, as in CPython.
# steelix-test: interrupt after the first line of output
print("ready", flush=True)
n = 0
while True:
    n += 1
