"""An exception caught by a handler that does not use it is released when the
handler ends, like in CPython, not when the function returns: its `__del__`
runs, and weakrefs to its arguments die, before the next statement (design
§9). Raised here, raised by a callee, with builtin and user classes."""

import weakref


class Big(Exception):
    def __del__(self):
        print("del", self.args[0])


class Arg:
    def __init__(self, tag):
        self.tag = tag


def gone(ref):
    print("weakref gone")


def raise_big(tag):
    raise Big(tag)


def empty_handler():
    try:
        raise Big("empty")
    except Big:
        pass
    print("after empty handler")


def builtin_class():
    arg = Arg("value-error")
    ref = weakref.ref(arg, gone)
    try:
        raise ValueError(arg)
    except ValueError:
        pass
    del arg
    print("after builtin handler", ref() is None)


def from_callee():
    try:
        raise_big("callee")
    except Big:
        pass
    print("after callee handler")


def handler_continue():
    for i in range(2):
        try:
            raise Big(f"continue {i}")
        except Big:
            continue
    print("after loop")


def handler_break():
    while True:
        try:
            raise Big("break")
        except Big:
            break
    print("after break")


def reraise_caught_outside():
    try:
        try:
            raise Big("inner")
        except Big:
            raise
    except Big:
        pass
    print("after nested")


def finally_then_handler():
    try:
        try:
            raise Big("finally")
        finally:
            print("finally runs")
    except Big:
        pass
    print("after finally")


def handler_as():
    try:
        raise Big("as")
    except Big as err:
        print("handling", err)
    print("after as")


def frame_names(err):
    names = []
    tb = err.__traceback__
    while tb is not None:
        names.append(tb.tb_frame.f_code.co_name)
        tb = tb.tb_next
    return names


def address_reused():
    # The exception from the callee usually gets the address of the one
    # raised here, which is gone by then; it still gets this frame in its
    # traceback.
    for i in range(3):
        try:
            raise Big(f"here {i}")
        except Big:
            pass
        try:
            raise_big(f"callee {i}")
        except Big as err:
            print("traceback", frame_names(err))


def leaves():
    try:
        raise Big("leaves")
    except Big:
        pass
    raise_big("uncaught here")


for case in (
    empty_handler,
    builtin_class,
    from_callee,
    handler_continue,
    handler_break,
    reraise_caught_outside,
    finally_then_handler,
    handler_as,
    address_reused,
):
    case()
    print("returned from", case.__name__)

try:
    leaves()
except Big as err:
    print("caught", err)
print("end")
