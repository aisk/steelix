"""A multi-module test application (tests/test_projects.py).

    python app.py          runs everything and exits normally
    python app.py crash    ends with an exception raised by code kept as
                           Python in a submodule, through native frames of
                           other modules
"""

import importlib
import sys

print("app: start", __name__)

import config
import store
from store import Inventory, Item, total_value
from store.errors import OutOfStock, StoreError
from store.reports import render

print("app: imported store")

try:
    from . import config as relative_config
except ImportError as err:
    print("app: relative import:", err)

try:
    from store import nothing
except ImportError as err:
    print("app: missing name:", str(err).replace(__file__.rsplit("/", 1)[0], "<dir>"))


class Discounted(Item):
    """A subclass of a class from another module."""

    def __init__(self, name, price, quantity, percent):
        super().__init__(name, price, quantity)
        self.percent = percent

    def unit_price(self):
        return super().unit_price() * (100 - self.percent) // 100


def build():
    inventory = Inventory()
    inventory.add(Item("apple", 3, 10))
    inventory.add(Discounted("pear", 10, 4, 25))
    inventory.add(store.items.Perishable("milk", 2, 5, days=3))
    return inventory


def shop(inventory):
    try:
        inventory.take("pear", 10)
    except OutOfStock as err:
        print("app: caught", type(err).__module__, type(err).__qualname__, err, err.wanted)
    try:
        inventory.take("plum", 1)
    except StoreError as err:
        print("app: caught base class", type(err).__name__, err)
    inventory.take("apple", 4)


def plugins():
    # A computed name: transpiled only with `--include plugins`.
    for name in ["greet", "count"]:
        plugin = importlib.import_module("plugins." + name)
        print("app: plugin", plugin.__name__, plugin.run(), plugin.__loader__.name)
    static = importlib.import_module("plugins.static")
    print("app: plugin", static.__name__, static.run())


def state():
    print("app: shared state", config.settings, config.bump(), config.bump())
    config.settings["currency"] = "USD"
    print("app: label after change", store.items.label(Item("fig", 7, 1)))
    # Replacing a function of another module: its direct callers see it.
    original = store.items.fmt_price
    store.items.fmt_price = lambda price: f"<{price}>"
    print("app: patched", store.items.label(Item("fig", 7, 1)))
    store.items.fmt_price = original
    reloaded = importlib.reload(config)
    print("app: reloaded", reloaded is config, config.counter, config.settings)


def lazy():
    print("app: before lazy import")
    import lazy

    print("app: after lazy import", lazy.VALUE, lazy.app_is_main)


def main():
    inventory = build()
    shop(inventory)
    print("app: total", total_value(inventory))
    print(render(inventory))
    plugins()
    state()
    lazy()
    import legacy

    print("app: legacy", legacy.where(), legacy.__loader__.name)
    print("app: modules", sorted(m for m in sys.modules if m.split(".")[0] in {"store", "plugins", "config", "lazy", "app"}))
    if sys.argv[1:] == ["crash"]:
        inventory.add(Item("bad", 1, -1))
        print(render(inventory))


if __name__ == "__main__":
    main()
