"""Method level mixed mode: methods and class body statements that can not
be lowered stay Python, in the class namespace, and still see `super()`,
`__class__` and the class's private names."""


class Base:
    def greet(self):
        return "base"

    def items(self):
        return [1, 2, 3]


class Mixed(Base):
    native_before = 1
    squares = list(i * j for i in range(4) for j in [i])
    native_after = squares[-1]

    def greet(self):
        return "native:" + super().greet()

    def kept_greet(self):
        values = list(v for v in super().items() for _ in "1")
        return f"kept:{super().greet()}:{values}:{__class__.__name__}"

    @staticmethod
    def kept_static(n):
        return sum(i for i in range(n) for _ in "1")

    def __private(self):
        return next(s for s in ("private",) for _ in "1")

    def call_private(self):
        return self.__private()

    async def kept_async(self):
        return "async"

    def uses_kept(self):
        return self.kept_greet() + "|" + str(self.kept_static(4))

    lookup = dict(kv for kv in [("a", 1)] for _ in "1")

    def failing(self):
        return list(1 / 0 for _ in range(1) for _ in "1")


m = Mixed()
print(m.greet(), m.kept_greet(), m.uses_kept(), m.call_private())
print(Mixed.squares, Mixed.native_after, Mixed.lookup, Mixed.kept_static(5))
print(Mixed.kept_greet.__qualname__, Mixed.kept_greet.__code__.co_firstlineno)
print(type(Mixed.__dict__["kept_static"]).__name__, Mixed.__static_attributes__)
print(Mixed.__firstlineno__, Mixed.__qualname__, Mixed.__module__)
print(sorted(k for k in Mixed.__dict__ if not k.startswith("__")))
coro = m.kept_async()
try:
    coro.send(None)
except StopIteration as stop:
    print(stop.value)


class KeptSub(Mixed):
    def greet(self):
        return next(s for s in ["sub:" + super().greet()] for _ in "1")


class NativeSub(KeptSub):
    def greet(self):
        return "nsub:" + super().greet()


print(NativeSub().greet(), isinstance(NativeSub(), Mixed), issubclass(KeptSub, Base))


class Outer:
    class Inner:
        def kept(self):
            return next(s for s in [type(self).__qualname__] for _ in "1") + ":" + __class__.__qualname__

        def __hidden(self):
            return "hidden"

        def kept_hidden(self):
            return next(s for s in [self.__hidden()] for _ in "1")


print(Outer.Inner().kept(), Outer.Inner().kept_hidden(), Outer.Inner.kept.__qualname__)


def decorate(cls):
    class Wrapper(cls):
        def kept(self):
            return "wrapped"

    return Wrapper


@decorate
class Decorated:
    def kept(self):
        return next(s for s in [super().__repr__()[:9]] for _ in "1") + ":" + __class__.__name__


print(Decorated().kept(), Decorated.__mro__[1]().kept())


class WholeKept:
    if True:
        conditional: int = 1


print(WholeKept.conditional, WholeKept.__annotations__)

try:
    m.failing()
except ZeroDivisionError as err:
    import traceback

    tb = traceback.extract_tb(err.__traceback__)
    print([(f.name, f.lineno, f.line) for f in tb])
