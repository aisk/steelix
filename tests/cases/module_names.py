"""Module level functions named like the locals of the generated module code."""


def m(x):
    return x + 1


def py():
    return "py"


def globals(a, b=2):
    return a * b


def exec_module():
    return m(1) + globals(3)


def module(x):
    return x * 2


def match_length(text):
    # A local named like the context parameter of generated functions.
    m = len(text)
    return module(m)


print(m(1), py(), globals(4), exec_module(), match_length("abc"))
f = m
print(f(10), m.__name__, py.__name__, exec_module.__name__, module.__name__)
