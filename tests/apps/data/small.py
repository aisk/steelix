"""A small module for the tokenize and ast tools."""

import os


def greet(name: str, *, punct="!") -> str:
    # say hello
    return f"Hello, {name}{punct}"


class Greeter:
    def __call__(self, who):
        return greet(who)


if __name__ == "__main__":
    print(Greeter()(os.sep))
