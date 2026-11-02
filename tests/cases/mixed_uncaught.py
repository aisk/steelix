# Tracebacks through code kept as Python show its file and line numbers.


def native(values):
    return python_kept(values)


def python_kept(values):
    return [*values, 10 // values[-1]]


print(native([1, 2]))
print(native([1, 0]))
