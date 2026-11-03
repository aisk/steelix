# `try` bodies that can not raise: the handlers are dead code.


def constant():
    try:
        x = 1
    except Exception:
        x = 2
    return x


def local_copy(a):
    b = a
    try:
        c = b
    except ValueError as e:
        print("never", e)
        return None
    else:
        print("else runs")
    return c


def nothing():
    for i in range(3):
        try:
            pass
        except Exception:
            break
        else:
            continue
    return i


def pair(a, b):
    try:
        t = a, b
    except Exception:
        t = None
    finally:
        print("finally")
    return t


def returned(a):
    try:
        return a
    except NameError:
        return "unbound"


def maybe_unbound(flag):
    if flag:
        y = "set"
    try:
        z = y
    except UnboundLocalError:
        z = "unbound"
    return z


def deleted_in_loop(n):
    v = 0
    for _ in range(n):
        try:
            w = v
        except UnboundLocalError:
            print("unbound in loop")
            w = None
        del v
        v = w
    return v


def captured():
    k = 5

    def inner():
        try:
            return k
        except NameError:
            return None

    return inner()


print(constant(), local_copy(3), nothing(), pair(1, 2), returned("r"))
print(maybe_unbound(True), maybe_unbound(False), deleted_in_loop(2), captured())

try:
    g = 10
except Exception:
    g = 0
try:
    "docstring-like constant"
except Exception:
    print("never")
print(g)
