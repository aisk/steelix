# Traceback lines of decorators that raise when evaluated or applied, and of
# defaults that raise in a decorated def: CPython reports the line of the
# decorator (or of the default), not that of `def`.
import traceback


def boom(fn):
    raise ValueError("applying to " + fn.__qualname__)


def fails(label):
    raise ValueError("evaluating " + label)


def ok(fn):
    return fn


def show(exc):
    print(type(exc).__name__, exc)
    for frame in traceback.extract_tb(exc.__traceback__):
        print(" ", frame.name, frame.lineno, frame.line)


try:

    @ok
    @fails("second")
    @ok
    def evaluated():
        pass

except ValueError as e:
    show(e)

try:

    @ok
    @ok
    def defaulted(x=fails("default")):
        pass

except ValueError as e:
    show(e)

try:

    @ok
    @boom
    @ok
    def applied():
        pass

except ValueError as e:
    show(e)

try:

    @boom
    def single():
        pass

except ValueError as e:
    show(e)


class Holder:
    attr = None


holder = Holder()
holder.attr = Holder()
holder.attr.boom = boom

try:

    class InBody:
        @ok
        @holder.attr.boom
        def method(self):
            pass

except ValueError as e:
    show(e)

try:

    class Single:
        @staticmethod
        def fine():
            pass

        @boom
        def method(self):
            pass

except ValueError as e:
    show(e)


def nested():
    @ok
    @fails("in nested")
    def inner():
        pass

    return inner


try:
    nested()
except ValueError as e:
    show(e)
