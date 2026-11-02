# Tracebacks through code kept as Python show its file and line numbers.


def native(values):
    return python_kept(values)


def python_kept(values):
    return [10 // v for v in values]


print(native([1, 2]))
print(native([1, 0]))
