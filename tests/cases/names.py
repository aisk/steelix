"""Python names that are Rust keywords or names the generated code uses."""


def type(match, loop):
    fn = match + loop
    return fn


def mod(py, globals, self):
    let = py * 100 + globals * 10 + self
    return let


def exec_module(Some):
    return Some + 1


def maybe(flag):
    if flag:
        value = "set"
    return value


print(type(1, 2), mod(1, 2, 3), exec_module(41))
print(maybe(True))
alias = type
print(alias(3, 4))
