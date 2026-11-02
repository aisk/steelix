"""A tiny tokenizer and recursive-descent calculator with variables and functions."""

import math
import re

TOKEN_RE = re.compile(r"\s*(?:(\d+\.\d*|\d+)|([A-Za-z_]\w*)|(\*\*|[-+*/%(),=]))")

FUNCS = {"sqrt": math.sqrt, "max": max, "min": min, "abs": abs}


def tokenize(src):
    tokens = []
    pos = 0
    src = src.rstrip()
    while pos < len(src):
        m = TOKEN_RE.match(src, pos)
        assert m is not None, f"bad input at {pos}: {src[pos:]!r}"
        num, name, op = m.groups()
        if num is not None:
            tokens.append(("num", float(num) if "." in num else int(num)))
        elif name is not None:
            tokens.append(("name", name))
        else:
            tokens.append(("op", op))
        pos = m.end()
    tokens.append(("end", None))
    return tokens


def evaluate(src, env):
    tokens = tokenize(src)
    pos = 0

    def peek():
        return tokens[pos]

    def take(expected=None):
        nonlocal pos
        kind, value = tokens[pos]
        if expected is not None:
            assert value == expected, f"expected {expected}, got {value}"
        pos += 1
        return kind, value

    def expr():
        left = term()
        while peek()[1] in ("+", "-"):
            _, op = take()
            right = term()
            left = left + right if op == "+" else left - right
        return left

    def term():
        left = power()
        while peek()[1] in ("*", "/", "%"):
            _, op = take()
            right = power()
            if op == "*":
                left *= right
            elif op == "/":
                left /= right
            else:
                left %= right
        return left

    def power():
        base = unary()
        if peek()[1] == "**":
            take()
            return base ** power()
        return base

    def unary():
        if peek()[1] == "-":
            take()
            return -unary()
        return atom()

    def atom():
        kind, value = take()
        if kind == "num":
            return value
        if kind == "name":
            if peek()[1] == "(":
                take("(")
                args = [expr()]
                while peek()[1] == ",":
                    take()
                    args.append(expr())
                take(")")
                return FUNCS[value](*args)
            return env[value]
        assert value == "(", f"unexpected token {value!r}"
        inner = expr()
        take(")")
        return inner

    if len(tokens) > 2 and tokens[0][0] == "name" and tokens[1][1] == "=":
        pos = 2
        env[tokens[0][1]] = expr()
        return None
    return expr()


def main():
    env = {"pi": math.pi}
    program = [
        "1 + 2 * 3",
        "(1 + 2) * 3",
        "2 ** 3 ** 2",
        "-4 + 10 % 3",
        "x = 7",
        "y = x * 2 - 1",
        "x * y",
        "sqrt(16) + max(1, y, 3)",
        "r = 2.5",
        "pi * r ** 2",
        "7 / 2",
        "min(abs(-3), 9)",
    ]
    for line in program:
        result = evaluate(line, env)
        if result is None:
            print(f"{line:<24} ok")
        elif isinstance(result, float):
            print(f"{line:<24} = {result:.4f}")
        else:
            print(f"{line:<24} = {result}")
    print(sorted(env))


if __name__ == "__main__":
    main()
