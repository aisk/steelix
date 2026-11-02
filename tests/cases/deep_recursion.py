# steelix-test: compare-exception-type-only
# CPython reports "maximum recursion depth exceeded"; transpiled code is only
# bounded by the C stack and reports "Stack overflow (used N kB)".


def depth(n):
    return depth(n + 1)


print("start")
depth(0)
