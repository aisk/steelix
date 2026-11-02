"""Class annotations with `from __future__ import annotations` are strings in
`__annotations__`, created at the start of the body."""

from __future__ import annotations

import dataclasses
import typing


class Meta(type):
    def __new__(mcls, name, bases, ns):
        print(name, [k for k in ns if k != "__module__"])
        return super().__new__(mcls, name, bases, ns)


class Config(metaclass=Meta):
    """Settings."""

    name: str = "app"
    size: int
    ratio: float = name and 0.5

    def scaled(self, by: int) -> Config:
        return self


print(Config.__annotations__, Config.name, Config.ratio, Config.scaled.__annotations__)
print(typing.get_type_hints(Config))


@dataclasses.dataclass
class Pair:
    left: int
    right: int = 0


print(Pair(1), Pair.__annotations__)


class Private:
    # The strings keep the names as written, the keys are mangled.
    __x: __T = 1
    y: list[__T]

    def f(self, __a: __T) -> __T:
        return __a


print(Private.__annotations__, Private.f.__annotations__)
