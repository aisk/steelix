"""How a class statement runs: order of evaluation, metaclasses,
`__prepare__`, `__mro_entries__`, `__init_subclass__`, `__set_name__`."""


def trace(label, value=None):
    print("eval", label)
    return value


def deco(label):
    print("deco", label)

    def apply(cls):
        print("apply", label, cls.__name__)
        cls.decorated = getattr(cls, "decorated", ()) + (label,)
        return cls

    return apply


class Base:
    pass


@deco("outer")
@deco("inner")
class Ordered(trace("base", Base), metaclass=trace("meta", type)):
    print("body start", __name__, __module__, __qualname__)
    x = trace("x", 1)
    y = x + 1
    print("body end", x, y)


print(Ordered.decorated, Ordered.x, Ordered.y, Ordered.__firstlineno__)


class Logged(dict):
    def __setitem__(self, key, value):
        print("set", key)
        super().__setitem__(key, value)


class Meta(type):
    @classmethod
    def __prepare__(mcls, name, bases, **kwargs):
        print("prepare", name, [b.__name__ for b in bases], kwargs)
        return Logged()

    def __new__(mcls, name, bases, ns, **kwargs):
        print("new", name, sorted(kwargs))
        return super().__new__(mcls, name, bases, dict(ns))

    def __init__(cls, name, bases, ns, **kwargs):
        print("init", name)
        super().__init__(name, bases, ns)


class WithMeta(metaclass=Meta, flag=True):
    """Doc."""

    a = 1

    def method(self):
        return super().__repr__()[:9]


print(type(WithMeta).__name__, WithMeta.a, WithMeta().method())


class Plugin:
    registry = []

    def __init_subclass__(cls, /, name=None, **kwargs):
        super().__init_subclass__(**kwargs)
        cls.plugin_name = name or cls.__name__.lower()
        Plugin.registry.append(cls.plugin_name)


class Csv(Plugin, name="csv"):
    pass


class Json(Plugin):
    pass


print(Plugin.registry, Csv.plugin_name, Json.plugin_name)


class Field:
    def __set_name__(self, owner, name):
        print("set_name", owner.__name__, name)
        self.name = name

    def __get__(self, obj, objtype=None):
        if obj is None:
            return self
        return obj.__dict__.get(self.name, "unset")


class Record:
    title = Field()
    body = Field()


r = Record()
r.__dict__["title"] = "hello"
print(r.title, r.body, Record.title.name)


class Alias:
    def __mro_entries__(self, bases):
        print("mro_entries", len(bases))
        return (Base,)


alias = Alias()


class FromAlias(alias):
    pass


print(FromAlias.__bases__ == (Base,), FromAlias.__orig_bases__ == (alias,))

bases = (Base,)
extra = {"metaclass": type}


class Starred(*bases, **extra):
    pass


print(Starred.__bases__ == (Base,))


def errors():
    try:

        class Bad(*5):
            pass

    except TypeError as err:
        print("TypeError:", err)
    try:

        class Bad2(**5):
            pass

    except TypeError as err:
        print("TypeError:", err)
    try:

        class Bad3(metaclass=type, **{"metaclass": type}):
            pass

    except TypeError as err:
        print("TypeError:", err)


class Errors:
    try:

        class Bad(*5):
            pass

    except TypeError as err:
        print("TypeError:", err)
    try:

        class Bad2(**5):
            pass

    except TypeError as err:
        print("TypeError:", err)
    try:

        class Bad3(metaclass=type, **{"metaclass": type}):
            pass

    except TypeError as err:
        print("TypeError:", err)


class M1(type):
    pass


class M2(type):
    pass


class A1(metaclass=M1):
    pass


class A2(metaclass=M2):
    pass


try:

    class Conflict(A1, A2):
        pass

except TypeError as err:
    print("TypeError:", err)


class BadEntries:
    def __mro_entries__(self, bases):
        return [Base]


try:

    class Bad4(BadEntries()):
        pass

except TypeError as err:
    print("TypeError:", err)


class BadPrepare(type):
    @classmethod
    def __prepare__(mcls, name, bases):
        return 42


try:

    class Bad5(metaclass=BadPrepare):
        pass

except TypeError as err:
    print("TypeError:", err)


class DropCell(type):
    def __new__(mcls, name, bases, ns):
        ns = {k: v for k, v in ns.items() if k != "__classcell__"}
        return super().__new__(mcls, name, bases, ns)


try:

    class Bad6(metaclass=DropCell):
        def method(self):
            return __class__

except RuntimeError as err:
    print("RuntimeError:", err)


try:

    class Bad7(metaclass=5):
        pass

except TypeError as err:
    print("TypeError:", err)


def fake_meta(name, bases, ns, **kwargs):
    print("fake_meta", name, bases, sorted(k for k in ns if not k.startswith("__")), kwargs)
    return 42


class NotAClass(metaclass=fake_meta, option=1):
    value = 1


print(NotAClass)


if Ordered.x:

    class Conditional:
        branch = "then"

else:

    class Conditional:
        branch = "else"


print(Conditional.branch)


class Outer:
    class Inner:
        class Deepest:
            def where(self):
                return type(self).__qualname__

    def make(self):
        return self.Inner.Deepest()


print(Outer.Inner.__qualname__, Outer().make().where(), Outer.Inner.Deepest.where.__qualname__)

for i in range(2):

    class InLoop:
        index = i

    print(InLoop.index, InLoop.__qualname__)


def replaced(cls):
    class Replacement(cls):
        def hello(self):
            return "replacement+" + super().hello()

    return Replacement


@replaced
class Original:
    def hello(self):
        return "original:" + __class__.__name__

    def via_super(self):
        return super().__repr__()[:10]


print(Original().hello(), Original.__name__, Original().via_super())
