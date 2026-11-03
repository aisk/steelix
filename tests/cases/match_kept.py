"""OR patterns with captures stay Python (design §13): the function or the
module level statement containing one is kept, other matches are native."""


def native(value):
    match value:
        case [x, y]:
            return f"pair {x} {y}"
        case 0 | 1:
            return "bit"
    return "other"


def kept(value):
    match value:
        case [x, _] | {"x": x}:
            return f"x is {x}"
        case (0 | 1) as bit:
            return f"bit {bit}"
    return "other"


for value in [[1, 2], {"x": 3}, 0, 5]:
    print(native(value), "/", kept(value))

# A module level loop keeps only the `match` that contains the OR pattern.
for value in [("add", 1), ("sub", 2), ("mul", 3)]:
    match value:
        case ("add", n) | ("sub", n):
            print("additive", n)
        case _:
            print("other", value)
    print("native", value)
print(n)
