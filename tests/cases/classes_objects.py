"""Instances of transpiled classes behave like Python ones: pickling,
copying, `__slots__`, comparisons and `isinstance`."""

import copy
import pickle


class Point:
    def __init__(self, x, y):
        self.x = x
        self.y = y

    def __eq__(self, other):
        return isinstance(other, Point) and (self.x, self.y) == (other.x, other.y)

    __hash__ = None

    def __repr__(self):
        return f"Point({self.x!r}, {self.y!r})"


class Slots:
    __slots__ = ("a", "b")

    def __init__(self, a):
        self.a = a

    def __reduce__(self):
        return (Slots, (self.a,))


class Tree:
    def __init__(self, value, *children):
        self.value = value
        self.children = list(children)

    def __deepcopy__(self, memo):
        print("deepcopy", self.value)
        return Tree(self.value, *copy.deepcopy(self.children, memo))

    def total(self):
        return self.value + sum(child.total() for child in self.children) if False else (
            self.value + sum(map(Tree.total, self.children))
        )


p = Point(1, [2, 3])
q = pickle.loads(pickle.dumps(p))
print(q, q == p, q is p, type(q) is Point)
r = copy.deepcopy(p)
r.y.append(4)
print(p, r, copy.copy(p).y is p.y)
try:
    hash(p)
except TypeError as err:
    print("TypeError:", err)
s = Slots(5)
print(s.a, hasattr(s, "__dict__"), pickle.loads(pickle.dumps(s)).a, Slots.__slots__)
try:
    s.c = 1
except AttributeError as err:
    print("AttributeError:", err)
t = Tree(1, Tree(2), Tree(3, Tree(4)))
u = copy.deepcopy(t)
print(u.total(), u.children[1].children[0].value, u is not t)
print(pickle.loads(pickle.dumps(Point.__init__)) is Point.__init__)
print(isinstance(p, Point), isinstance(s, Point), issubclass(Point, object), type(p).__mro__)
