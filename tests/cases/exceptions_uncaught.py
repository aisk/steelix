# An uncaught exception with a cause and a context, raised through native
# handlers, `finally` and `with`: CPython prints all three tracebacks.
import contextlib


@contextlib.contextmanager
def resource(name):
    print("open", name)
    yield name
    print("close", name)


def load(key):
    table = {"a": 1}
    return table[key]


def lookup(key):
    try:
        return load(key)
    except KeyError as e:
        raise LookupError("no such key: " + key) from e


def process(keys):
    total = 0
    with resource("db"):
        for key in keys:
            try:
                total += lookup(key)
            finally:
                print("visited", key)
    return total


def main():
    try:
        process(["a", "b"])
    except LookupError:
        cleanup = None
        cleanup.close()


main()
