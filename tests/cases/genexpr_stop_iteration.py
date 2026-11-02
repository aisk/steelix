"""A `StopIteration` raised in a generator expression becomes a
`RuntimeError` (PEP 479); the traceback chains both."""


def boom(v):
    if v == 1:
        raise StopIteration("inner")
    return v


def run():
    return list(boom(v) for v in range(3))


run()
