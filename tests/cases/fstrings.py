"""f-strings with format specs, conversions and nested fields."""


def describe(name, value, width):
    print(f"{name}: {value}")
    print(f"{value:>8}|{value:<{width}}|{value:^{width}.2f}")
    print(f"{name!r} {name!s} {name!a} {name=} {value = :.1f}")
    print(f"{value:{'>' if value > 0 else '<'}{width}}")
    print(f"{{literal}} {name}{name}")
    print(f"{'é'!a}", f"", f"plain")
    return f"{name.upper()}={value * 2:08.3f}"


print(describe("pi", 3.14159, 10))
print(describe("ünïcode", -1.5, 6))
n = 255
print(f"{n:x} {n:#o} {n:,} {n:b}", f"{n}")
