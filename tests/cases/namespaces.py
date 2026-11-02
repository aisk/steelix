"""`dir()`, `vars()` and `locals()` without arguments read the current scope."""

x = 1
print("x" in dir(), dir() == sorted(globals()))
print(vars() is globals(), locals() is globals())


def local_names(a):
    b = a + 1
    return sorted(locals()), dir()


print(local_names(1))
