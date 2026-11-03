print("store.inventory: start")

from . import audit
from .errors import OutOfStock, StoreError


class Inventory:
    def __init__(self):
        self.items = {}

    def add(self, item):
        self.items[item.name] = item
        audit.record("add", item)

    def take(self, name, quantity):
        if name not in self.items:
            raise StoreError(f"unknown item {name}")
        item = self.items[name]
        if item.quantity < quantity:
            raise OutOfStock(name, quantity)
        item.quantity -= quantity
        audit.record("take", item)


print("store.inventory: end")
