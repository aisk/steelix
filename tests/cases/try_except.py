# try/except in native functions: matching, `as` names, `else`, chaining.
import sys


class AppError(Exception):
    pass


class SubError(AppError):
    pass


def parse(s):
    try:
        x = int(s)
    except ValueError:
        x = 0
    return x


def parse_or_none(s):
    try:
        return int(s)
    except (ValueError, TypeError):
        return None


def describe(exc):
    print(type(exc).__name__, exc.args, "cause:", repr(exc.__cause__),
          "context:", repr(exc.__context__), "suppress:", exc.__suppress_context__)


def risky(kind):
    if kind == "value":
        raise ValueError("bad value")
    if kind == "key":
        raise KeyError(kind)
    if kind == "app":
        raise SubError("sub", 2)
    if kind == "class":
        raise AppError
    return kind


def classify(kind):
    try:
        result = risky(kind)
    except KeyError as e:
        print("key error", e)
        return "key"
    except (ValueError, AppError) as e:
        print("caught", type(e).__name__, e.args)
        print("sys.exception() is e:", sys.exception() is e)
        return "other"
    else:
        print("no error, result", result)
        return "ok"
    finally:
        print("finally for", kind)


def chained():
    try:
        try:
            risky("value")
        except ValueError as e:
            raise RuntimeError("wrapped") from e
    except RuntimeError as e:
        describe(e)
    try:
        try:
            risky("key")
        except KeyError:
            raise RuntimeError("implicit")
    except RuntimeError as e:
        describe(e)
    try:
        try:
            risky("app")
        except AppError:
            raise TypeError("hidden") from None
    except TypeError as e:
        describe(e)


def reraise():
    try:
        risky("value")
    except ValueError as e:
        e.args = ("changed",)
        raise


def bare_reraise_helper():
    raise


def via_helper():
    try:
        risky("key")
    except KeyError:
        bare_reraise_helper()


def no_active():
    try:
        raise
    except RuntimeError as e:
        print("RuntimeError:", e)


def not_an_exception():
    try:
        raise 42
    except TypeError as e:
        print("TypeError:", e)
    try:
        raise ValueError from 3
    except TypeError as e:
        print("TypeError:", e)
    try:
        try:
            pass
        finally:
            risky("value")
    except int:
        print("unreachable")


def bad_handler():
    try:
        not_an_exception()
    except TypeError as e:
        print("bad handler:", e)


def handled_state():
    print("outside:", sys.exception())
    try:
        risky("value")
    except ValueError:
        print("inside:", repr(sys.exception()))
        try:
            risky("key")
        except KeyError:
            print("nested:", repr(sys.exception()))
        print("back:", repr(sys.exception()))
    print("after:", sys.exception())


def handler_name_is_deleted():
    try:
        risky("value")
    except ValueError as err:
        print("bound:", err)
    return "done"


def bare_except_catches_everything(exc):
    try:
        raise exc
    except Exception:
        return "Exception"
    except:
        return "bare"


def exceptions_in_else_and_handlers():
    try:
        try:
            print("body")
        except ValueError:
            print("not reached")
        else:
            risky("key")
    except KeyError as e:
        print("from else:", repr(e))
    try:
        try:
            risky("value")
        except ValueError:
            risky("key")
    except KeyError as e:
        print("from handler:", repr(e), "context:", repr(e.__context__))


def stop_iteration():
    it = iter([1])
    try:
        next(it)
        next(it)
    except StopIteration:
        print("exhausted")


def gen():
    yield 1
    raise ValueError("from generator")


def generator_into_native():
    total = 0
    try:
        for value in gen():
            total = total + value
    except ValueError as e:
        print("generator raised", e, "after", total)


def loop_with_try(items):
    total = 0
    for item in items:
        try:
            total = total + int(item)
        except ValueError:
            continue
        if total > 10:
            break
    return total


def groups():
    # `except*` is kept as Python.
    try:
        raise ExceptionGroup("g", [ValueError(1), KeyError(2)])
    except* ValueError as eg:
        print("values", eg.exceptions)
    except* KeyError as eg:
        print("keys", eg.exceptions)


def name_after_handler():
    # `e` is used after its handler: the function is kept as Python.
    try:
        raise ValueError("x")
    except ValueError as e:
        pass
    try:
        print(e)
    except NameError as err:
        print(type(err).__name__, err)


print(parse("12"), parse("x"))
print(parse_or_none("7"), parse_or_none(None), parse_or_none("y"))
for kind in ["value", "key", "app", "class", "fine"]:
    print(classify(kind))
chained()
try:
    reraise()
except ValueError as e:
    print("reraised", e.args)
try:
    via_helper()
except KeyError as e:
    print("helper reraised", repr(e))
no_active()
bad_handler()
handled_state()
print(handler_name_is_deleted())
print(bare_except_catches_everything(ValueError()))
print(bare_except_catches_everything(KeyboardInterrupt()))
print(bare_except_catches_everything(SystemExit(3)))
exceptions_in_else_and_handlers()
stop_iteration()
generator_into_native()
print(loop_with_try(["1", "x", "4", "y", "9", "100"]))
try:
    risky("app")
except AppError as e:
    print("module level", type(e).__name__, e.args)
print("e" in globals())
groups()
name_after_handler()
