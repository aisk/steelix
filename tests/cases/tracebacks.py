# Traceback entries of native functions: lines, names, and the order of
# native and kept (Python) frames, printed by the traceback module.
import io
import logging
import sys
import traceback

log_output = io.StringIO()
logging.basicConfig(stream=log_output, format="%(levelname)s %(message)s")


def show(exc):
    for frame in traceback.extract_tb(exc.__traceback__):
        print(" ", frame.name, frame.lineno, frame.line)


def no_carets(text):
    # Native frames have no column positions, so no `^^^` markers.
    return "\n".join(
        line for line in text.splitlines() if line.strip().strip("^~") != ""
    )


def leaf(values, key):
    total = 0
    for value in values:
        total = total + value
    return values[key] + total


def kept_middle(values, key):
    pairs = list(p for p in enumerate(values) for _ in "1")
    return leaf(values, key) + len(pairs)


def native_middle(values, key):
    adjust = 1
    result = kept_middle(values, key)
    return result + adjust


def decorate(fn):
    return fn


@decorate
def decorated(x):
    return 1 / x


def outer():
    def inner(n):
        return [1][n]

    square = lambda v: v * v
    try:
        inner(5)
    except IndexError as e:
        show(e)
    try:
        square("text")
    except TypeError as e:
        show(e)
    return inner


def multiline(a, b):
    return (
        a
        + b
    )


def raise_again():
    try:
        leaf([1], 3)
    except IndexError as e:
        raise e


def raise_from_handler():
    try:
        leaf([1], 3)
    except IndexError:
        raise ValueError("converted")


def in_loop(items):
    for item in items:
        if item:
            count = len(item)
        else:
            count = 0
    while count < 3:
        count = count + 1
        if count == 2:
            continue
    return items[count]


try:
    native_middle([1, 2], 7)
except IndexError as e:
    show(e)
    out = io.StringIO()
    traceback.print_exc(file=out)
    print(no_carets(out.getvalue()))
    logging.exception("logged")
    print(no_carets(log_output.getvalue()))

try:
    decorated(0)
except ZeroDivisionError as e:
    show(e)

outer()

try:
    multiline(1, "x")
except TypeError as e:
    show(e)

for fn in [raise_again, raise_from_handler]:
    try:
        fn()
    except Exception as e:
        out = io.StringIO()
        traceback.print_exception(e, file=out)
        print(no_carets(out.getvalue()))

try:
    in_loop(["ab", ""])
except IndexError as e:
    show(e)


def main():
    native_middle([3], 9)


main()
