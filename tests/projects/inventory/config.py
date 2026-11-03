"""Module level state shared between modules."""

print("config: imported as", __name__)

settings = {"currency": "EUR"}
counter = 0


def bump():
    global counter
    counter += 1
    return counter


try:
    from .. import anything
except ImportError as err:
    print("config: relative import:", err)

if __name__ == "__main__":
    print("config: run as a script")
else:
    print("config: not the main module")
