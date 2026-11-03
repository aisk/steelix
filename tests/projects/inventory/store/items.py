import config

print("store.items: imported")


def fmt_price(price):
    return f"{price} {config.settings['currency']}"


def label(item):
    return f"{item.name}: {fmt_price(item.unit_price())}"


class Item:
    def __init__(self, name, price, quantity):
        self.name = name
        self.price = price
        self.quantity = quantity

    def unit_price(self):
        return self.price

    def value(self):
        return self.unit_price() * self.quantity

    def __repr__(self):
        return f"{type(self).__name__}({self.name!r}, {self.quantity})"


class Perishable(Item):
    def __init__(self, name, price, quantity, days):
        super().__init__(name, price, quantity)
        self.days = days
