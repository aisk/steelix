"""Private names are mangled like CPython does."""


class Account:
    __count = 0
    __slots_like = ("__a", "__b")

    def __init__(self, owner):
        self.__owner = owner
        self.__class__.__count += 1
        Account.__count += 0

    def __secret(self):
        return "secret of " + self.__owner

    def reveal(self):
        return self.__secret()

    def get(self, __key="default"):
        return __key

    @staticmethod
    def count():
        return Account.__count

    class __Inner:
        __level = 2

        def level(self):
            return self.__level

    def inner(self):
        return self.__Inner().level()

    def handler(self):
        try:
            raise KeyError("k")
        except KeyError as __err:
            return repr(__err)

    def names(self):
        __local = 1
        return [n for n in dir(self) if "secret" in n or "owner" in n] + [__local]


class _Under:
    def __x(self):
        return "under"

    def call(self):
        return self.__x()


class ___:
    def __y(self):
        return "only underscores"

    def call(self):
        return self.__y()


a = Account("ann")
Account("bob")
print(a.reveal(), a._Account__owner, Account.count(), a.inner(), a.get(), a.get(_Account__key=1))
print(a.handler(), a.names(), hasattr(a, "__owner"), "_Account__secret" in Account.__dict__)
print(Account._Account__Inner.__qualname__, Account.__static_attributes__)
print(_Under().call(), ___().call(), sorted(k for k in ___.__dict__ if "y" in k))
print(Account._Account__slots_like)
