from __future__ import annotations


def native(x: int) -> str:
    return str(x)


def kept(x: int) -> Iterator[str]:
    yield str(x)


class Box:
    size: Size


print(native.__annotations__, kept.__annotations__, Box.__annotations__)
