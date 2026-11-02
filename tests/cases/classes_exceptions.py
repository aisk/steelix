"""Custom exception classes raised and caught by native and kept code."""


class AppError(Exception):
    """Base error."""


class ConfigError(AppError):
    def __init__(self, key, problem):
        super().__init__(f"{key}: {problem}")
        self.key = key
        self.problem = problem


class Retry(AppError):
    def __str__(self):
        return "retry " + ", ".join(map(str, self.args))


def load(key):
    if key == "missing":
        raise ConfigError(key, "not found")
    if key == "busy":
        raise Retry(1, 2)
    return key.upper()


for key in ["ok", "missing", "busy"]:
    try:
        print(load(key))
    except ConfigError as err:
        print("config:", err, err.key, err.problem, err.args, isinstance(err, AppError))
    except AppError as err:
        print("app:", err, repr(err), type(err).__mro__[1].__name__)

try:
    raise ConfigError("a", "b") from KeyError("k")
except AppError as err:
    print(repr(err.__cause__), err.__suppress_context__)


class Kept(Exception):
    def describe(self):
        return [len(a) for a in [self.args]]


try:
    raise Kept("x", "y")
except Kept as err:
    print(err.describe(), err.args)


class Broken:
    def __init__(self, value):
        if value < 0:
            raise ValueError("negative")
        self.value = value


try:
    Broken(-1)
except ValueError as err:
    print("ValueError:", err)
print(Broken(2).value)


class Grouped(ExceptionGroup):
    def derive(self, excs):
        return Grouped(self.message, excs)


g = Grouped("many", [ValueError(1), TypeError(2)])
match_, rest = g.split(ValueError)
print(type(match_).__name__, match_.exceptions, rest.exceptions)
