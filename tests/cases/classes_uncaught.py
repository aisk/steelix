"""An uncaught exception raised in a method: the traceback has the frames of
the methods and the module."""


class Account:
    def __init__(self, balance):
        self.balance = balance

    def withdraw(self, amount):
        if amount > self.balance:
            raise ValueError(f"insufficient funds: {self.balance} < {amount}")
        self.balance -= amount
        return self.balance

    @classmethod
    def opened(cls, balance):
        account = cls(balance)
        account.withdraw(balance * 2)
        return account


print("opening")
first = Account.opened(5)
