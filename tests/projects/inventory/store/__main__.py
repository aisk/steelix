"""`python -m store`."""

import sys

from . import Inventory, Item
from .reports import render

print("store.__main__:", __name__, __package__, __spec__.name, sys.argv[0].rsplit("/", 2)[-2:])
inventory = Inventory()
inventory.add(Item("kiwi", 2, 3))
print(render(inventory))
if sys.argv[1:] == ["crash"]:
    inventory.take("kiwi", 5)
