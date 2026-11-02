# A parameter named like its own function, as in `timeit.repeat(repeat=...)`.


def repeat(text, repeat=2):
    return text * repeat


def outer(inner):
    def inner(inner):
        return inner + 1

    return inner(1)


print(repeat("ab"), repeat("ab", repeat=3), outer(0))
print([repeat("x", n) for n in range(3)])
