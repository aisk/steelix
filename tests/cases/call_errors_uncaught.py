"""An uncaught error from a call that can not be bound at transpile time."""


def connect(host, port, *, timeout):
    return host


print(connect("localhost", 80, timeout=1))
connect("localhost", timeout=5)
