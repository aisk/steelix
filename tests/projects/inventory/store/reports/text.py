from .. import errors


def lines(inventory):
    # A generator function.
    for item in inventory.items.values():
        if item.quantity < 0:
            raise errors.StoreError(f"negative quantity of {item.name}")
        yield f"{item.name:>6} x{item.quantity:<3} {item.value():>4}"


def render(inventory):
    out = []
    for line in lines(inventory):
        out.append(line)
    return "\n".join(out)


if __name__ == "__main__":
    print("store.reports.text: run as a script")
else:
    print("store.reports.text: imported as", __name__)
