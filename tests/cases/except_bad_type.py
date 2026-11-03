# Errors raised while matching an `except` clause: a handler that is not an
# exception class, or a handler expression that raises.
import sys


def report(error):
    tb = error.__traceback__
    while tb.tb_next is not None:
        tb = tb.tb_next
    print(type(error).__name__, repr(error.__context__), tb.tb_lineno)


def bad_class():
    try:
        raise ValueError("x")
    except 42:
        pass


def missing_name():
    try:
        int("nope")
    except KeyError:
        print("not this one")
    except Missing:  # noqa: F821
        pass


def handled_while_testing():
    def pick():
        print("handling", repr(sys.exception()))
        return ValueError

    try:
        raise ValueError("seen")
    except pick() as e:
        print("caught", repr(e))
    print("after", sys.exception())


def tuple_with_bad_member(n):
    try:
        return 1 // n
    except (ArithmeticError, "text"):
        return None


def class_in_a_local(kinds):
    # The handler expression is evaluated each time, the local stays usable.
    caught = 0
    ignored = kinds[0]
    for kind in kinds:
        ignored = kind
        try:
            {}["k"]
        except ignored:
            caught += 1
    return caught, ignored.__name__


for test in (bad_class, missing_name, lambda: tuple_with_bad_member(0)):
    try:
        test()
    except Exception as error:
        report(error)
handled_while_testing()
print(tuple_with_bad_member(1))
print(class_in_a_local([KeyError, LookupError]))

try:
    [][1]
except IndexError if len(sys.argv) > 5 else 3.5: pass  # fmt: skip
