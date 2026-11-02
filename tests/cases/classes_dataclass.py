"""Class annotations (PEP 649) as dataclasses and `typing` see them."""

import annotationlib
import dataclasses
import typing
from dataclasses import dataclass, field


@dataclass
class Item:
    name: str
    price: float = 0.0
    tags: list = field(default_factory=list)

    def total(self, quantity: int) -> float:
        return self.price * quantity


@dataclass(frozen=True, order=True)
class Version:
    major: int
    minor: int = 0

    def bump(self) -> "Version":
        return Version(self.major, self.minor + 1)


item = Item("pen", 1.5)
print(item, item.total(3), Item.__annotations__, Item("a") == Item("a"))
v = Version(1)
print(v, v.bump(), sorted([Version(2), Version(1, 5), v]), v < v.bump())
try:
    v.major = 3
except dataclasses.FrozenInstanceError as err:
    print("FrozenInstanceError:", err)
print(hash(v) == hash(Version(1)), [f.name for f in dataclasses.fields(Version)])
print(typing.get_type_hints(Item.total), typing.get_type_hints(Version.bump))


class Node:
    value: int
    children: "list[Node]"
    parent: Node | None = None
    alias = int
    with_alias: alias

    def link(self, other: Node) -> alias:
        return 1


print(Node.__annotations__)
print(typing.get_type_hints(Node))
print(Node.link.__annotations__)
print(annotationlib.get_annotations(Node, format=annotationlib.Format.STRING))
print(Node.__annotate__(annotationlib.Format.VALUE) == Node.__annotations__)
print(Node.__annotate__.__qualname__, Node.parent)


class Forward:
    later: Later
    count: int


print(annotationlib.get_annotations(Forward, format=annotationlib.Format.FORWARDREF))
try:
    Forward.__annotations__
except NameError as err:
    print("NameError:", err)


class Later:
    pass


print(Forward.__annotations__)


@dataclass(slots=True)
class Slotted:
    x: int
    y: int = 2

    def total(self):
        return self.x + self.y


s = Slotted(1)
print(s, s.total(), Slotted.__slots__, hasattr(s, "__dict__"))


@dataclass
class Child(Item):
    color: str = "red"

    def total(self, quantity: int) -> float:
        return super().total(quantity) + 1


print(Child("x", 2.0), Child("x", 2.0).total(2))


class Typed(typing.Generic[typing.TypeVar("T")]):
    value: int = 3


print(Typed[int], Typed.__annotations__, Typed().value)
