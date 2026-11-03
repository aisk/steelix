print("store.errors: imported", __package__)


class StoreError(Exception):
    pass


class OutOfStock(StoreError):
    def __init__(self, name, wanted):
        super().__init__(f"only {name!r} left" if False else f"not enough {name}")
        self.wanted = wanted


try:
    from ... import beyond
except ImportError as err:
    print("store.errors:", err)
