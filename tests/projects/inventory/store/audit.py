"""Imported by `store.inventory` while that is still being initialized."""

print("store.audit: start")

from . import inventory  # circular: partially initialized, found in sys.modules

try:
    from .inventory import Inventory
except ImportError as err:
    print("store.audit:", str(err).replace(__file__.rsplit("/", 2)[0], "<dir>"))

try:
    inventory.Inventory
except AttributeError as err:
    print("store.audit:", err)

LOG = []


def record(action, item):
    LOG.append((action, item.name))
    if len(LOG) > 3:
        del LOG[0]
    # The module is complete by now.
    assert isinstance(inventory.Inventory, type)


print("store.audit: end")
