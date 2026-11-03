"""A package that re-exports names of its submodules."""

print("store: init start")

from .errors import OutOfStock, StoreError
from .items import Item, Perishable
from .inventory import Inventory

__all__ = ["Item", "Perishable", "Inventory", "StoreError", "OutOfStock", "total_value"]


def total_value(inventory):
    return sum(item.value() for item in inventory.items.values())


print("store: init end")
