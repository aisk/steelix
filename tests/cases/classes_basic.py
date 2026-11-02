"""Methods, properties, class and static methods, special methods."""

scale = 10


class Vector:
    """A 2D vector."""

    scale = 2
    instances = 0

    def __init__(self, x, y=0):
        self.x = x
        self.y = y
        Vector.instances += 1

    def __repr__(self):
        return f"Vector({self.x}, {self.y})"

    def __eq__(self, other):
        if not isinstance(other, Vector):
            return NotImplemented
        return (self.x, self.y) == (other.x, other.y)

    def __hash__(self):
        return hash((self.x, self.y))

    def __add__(self, other):
        return Vector(self.x + other.x, self.y + other.y)

    def scaled(self):
        # The global `scale`, not the class attribute: class scope is not
        # visible in methods.
        return Vector(self.x * scale, self.y * scale)

    def class_scaled(self):
        return Vector(self.x * self.scale, self.y * type(self).scale)

    @property
    def length(self):
        return (self.x**2 + self.y**2) ** 0.5

    @property
    def label(self):
        return self._label

    @label.setter
    def label(self, value):
        print("setting label", value)
        self._label = value

    @label.deleter
    def label(self):
        print("deleting label")
        del self._label

    @classmethod
    def zero(cls):
        return cls(0, 0)

    @staticmethod
    def dot(a, b):
        return a.x * b.x + a.y * b.y

    def __len__(self):
        return 2

    def __iter__(self):
        yield_values = [self.x, self.y]
        return iter(yield_values)

    def __getitem__(self, index):
        return (self.x, self.y)[index]

    def __bool__(self):
        return bool(self.x or self.y)


v = Vector(3, 4)
w = Vector(1)
print(v, w, v + w, v == Vector(3, 4), v != w, v == 3)
print(hash(v) == hash(Vector(3, 4)), len({v, Vector(3, 4), w}))
print(v.length, v.scaled(), v.class_scaled(), Vector.zero(), Vector.dot(v, w))
v.label = "a"
print(v.label)
del v.label
print(hasattr(v, "_label"), len(v), list(v), v[1], bool(v), bool(Vector.zero()))
print(Vector.instances, Vector.__doc__, Vector.__name__, Vector.__qualname__, Vector.__module__)
print(Vector.__init__.__qualname__, Vector.__init__.__name__, Vector.__init__.__defaults__)
print(type(Vector.__dict__["zero"]).__name__, type(Vector.__dict__["dot"]).__name__)
print(Vector.__static_attributes__, Vector.__firstlineno__)
print(sorted(k for k in Vector.__dict__ if not k.startswith("__")))
print(v.__dict__, vars(w))
bound = v.scaled
print(bound.__self__ is v, bound.__func__ is Vector.scaled, bound())


class Counter:
    count = 0

    def increment(self, by=1):
        self.count += by
        return self

    def __call__(self, *args, **kwargs):
        return (args, kwargs)

    def __str__(self):
        return "Counter(%d)" % self.count


c = Counter().increment().increment(5)
print(c, Counter.count, c.count, c(1, 2, k=3), str(c), repr(c)[:8])


class Empty:
    pass


e = Empty()
e.attr = 1
print(type(e).__name__, e.attr, Empty.__doc__, Empty.__bases__, Empty.__static_attributes__)
