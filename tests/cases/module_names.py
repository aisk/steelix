"""Module level functions named like the locals of the generated module code."""


def m(x):
    return x + 1


def py():
    return "py"


def globals(a, b=2):
    return a * b


def exec_module():
    return m(1) + globals(3)


print(m(1), py(), globals(4), exec_module())
f = m
print(f(10), m.__name__, py.__name__, exec_module.__name__)
