"""Decorators: evaluation and application order, functools helpers."""

import functools


def trace(label):
    print("decorator expression", label)

    def decorate(fn):
        print("apply", label, "to", fn.__name__)

        @functools.wraps(fn)
        def wrapper(*args, **kwargs):
            print("enter", label)
            result = fn(*args, **kwargs)
            print("leave", label)
            return result

        return wrapper

    return decorate


def default_value(label):
    print("default", label)
    return label


@trace("outer")
@trace("inner")
def greet(name, punct=default_value("!")):
    """Say hello."""
    return "hello " + name + punct


def register(fn):
    registry.append(fn.__name__)
    return fn


registry = []


@register
def first():
    return 1


@register
def second():
    return first() + 1


@functools.lru_cache(maxsize=None)
def fib(n):
    if n < 2:
        return n
    return fib(n - 1) + fib(n - 2)


def replace(fn):
    return "not a function any more"


@replace
def gone():
    pass


print(greet("world"))
print(greet.__name__, greet.__qualname__, greet.__doc__, greet.__wrapped__.__name__)
print(greet.__wrapped__.__wrapped__("x", "?"))
print(registry, second())
print(fib(80), fib.cache_info().hits)
print(gone)
