# try/finally: jumps through `finally`, `finally` overriding them, and
# exceptions raised or swallowed in `finally`.
import sys


def log(*args):
    print(*args)


def finally_overrides_return():
    try:
        return "try"
    finally:
        return "finally"


def finally_swallows_exception():
    try:
        raise ValueError("lost")
    finally:
        return "swallowed"


def break_in_finally_swallows():
    for i in range(3):
        try:
            raise KeyError(i)
        finally:
            break
    return i


def continue_in_finally(n):
    count = 0
    for i in range(n):
        try:
            if i % 2:
                raise ValueError(i)
            count += 1
        finally:
            continue
    return count


def return_in_finally_in_loop(items):
    for item in items:
        try:
            if item == "stop":
                return "stopped"
            log("item", item)
        finally:
            log("cleanup", item)
            if item == "late":
                return "late return"
    return "end"


def jumps_run_finally():
    out = []
    for i in range(5):
        try:
            if i == 1:
                continue
            if i == 3:
                break
            out.append(i)
        finally:
            out.append(-i)
    return out


def nested_finally():
    try:
        try:
            log("inner body")
            return "inner"
        finally:
            log("inner finally")
    finally:
        log("outer finally")


def exception_in_finally():
    try:
        try:
            raise ValueError("first")
        finally:
            raise KeyError("second")
    except KeyError as e:
        log("caught", repr(e), "context", repr(e.__context__))


def exception_state_in_finally():
    try:
        try:
            raise ValueError("pending")
        finally:
            log("in finally:", repr(sys.exception()))
    except ValueError:
        pass
    try:
        log("no exception")
    finally:
        log("in finally:", repr(sys.exception()))


def finally_after_handler():
    events = []
    try:
        events.append("body")
        raise IndexError("x")
    except IndexError:
        events.append("handler")
        return events
    else:
        events.append("else")
    finally:
        events.append("finally")


def assigned_in_try(flag):
    try:
        value = "set"
        if flag:
            raise RuntimeError("boom")
    except RuntimeError:
        value = "handled"
    finally:
        log("finally sees", value)
    return value


def while_with_finally():
    n = 0
    while n < 10:
        try:
            n += 1
            if n == 3:
                continue
            if n == 5:
                break
        finally:
            log("tick", n)
    else:
        log("no break")
    return n


print(finally_overrides_return())
print(finally_swallows_exception())
print(break_in_finally_swallows())
print(continue_in_finally(5))
print(return_in_finally_in_loop(["a", "late", "b"]))
print(return_in_finally_in_loop(["a", "stop", "b"]))
print(return_in_finally_in_loop(["a"]))
print(jumps_run_finally())
print(nested_finally())
exception_in_finally()
exception_state_in_finally()
print(finally_after_handler())
print(assigned_in_try(False), assigned_in_try(True))
print(while_with_finally())
try:
    log("module try")
finally:
    log("module finally")
for k in range(3):
    try:
        if k == 1:
            continue
        log("module loop", k)
    finally:
        log("module cleanup", k)
