# `match` statements (design §8): sequence patterns, literals, captures,
# guards, OR and AS patterns, at module level, in functions and in a class body.

EXITS = {"north", "south", "east"}


def command(text):
    match text.split():
        case []:
            return "nothing"
        case ["go", direction] if direction in EXITS:
            return f"going {direction}"
        case ["go", _]:
            return "no exit there"
        case ["look"]:
            return "you see a room"
        case ["take", *items] if items:
            return "taking " + ", ".join(items)
        case ["drop", item, *_]:
            return f"dropping {item}"
        case [verb, *_, last]:
            return f"{verb}? ({last})"
        case _:
            return "?"


for line in ["", "go north", "go west", "look", "take", "take lamp key", "drop a b c", "x y", "z"]:
    print(repr(line), "->", command(line))


def stars(value):
    """A named star at each position, and `*_` with the items around it."""
    match value:
        case [*rest]:
            print("all", rest)
    match value:
        case [first, *rest]:
            print("first", first, rest)
        case _:
            print("empty")
    match value:
        case [*init, last]:
            print("last", init, last)
        case _:
            print("no last")
    match value:
        case [a, *middle, b, c]:
            print("ends", a, middle, b, c)
        case [*_, 9]:
            print("ends with 9")
        case [0, *_]:
            print("starts with 0")
        case [_, *_, _, 1]:
            print("at least three, last 1")
        case _:
            print("other")


for value in [[], [9], [0], [1, 2], [1, 2, 3], (1, 2, 3, 4, 5), range(3), "abc", [5, 6, 1]]:
    stars(value)


def literals(value):
    match value:
        case None:
            return "None"
        case True:
            return "True"
        case False:
            return "False"
        case 1:
            return "one (== 1)"
        case 2.5:
            return "two and a half"
        case -3:
            return "minus three"
        case 1 + 2j:
            return "complex"
        case "text":
            return "the string"
        case b"bytes":
            return "the bytes"
        case "a" | "b" | "c":
            return "a letter"
        case 10 | 20 | 30 as tens:
            return f"tens {tens}"
        case (4 | 5) as n:
            return f"four or five: {n}"
    return "no match"


for value in [None, True, False, 1, 1.0, 0, 2.5, -3, -3.0, 1 + 2j, "text", b"bytes", "b", 20, 5, "x"]:
    print(repr(value), literals(value))


def nested(point):
    match point:
        case [(0, 0), label]:
            return f"origin {label}"
        case [(1, 0) | (0, 1), _]:
            return "unit"
        case [[x, y], [*tags]] if x == y:
            return f"diagonal {x} {tags}"
        case [[x, y], str() as label]:
            return f"point {x},{y} {label}"
        case [[_, _] as pair, _]:
            return f"pair {pair}"
    return "other"


for point in [[(0, 0), "o"], [(0, 1), "u"], [(2, 2), ["a", "b"]], [[1, 2], "p"], [(1, 2), 3], [1, 2]]:
    print(nested(point))


def guard_binds(values):
    """A guard that fails still leaves the captures bound (CPython stores
    them before the guard runs)."""
    match values:
        case [x, y] if x > y:
            print("descending")
        case [x, *_] if x < 0:
            print("negative start")
        case _:
            pass
    print("x after the match:", x)
    try:
        print(y)
    except UnboundLocalError as err:
        print("UnboundLocalError:", err)


guard_binds([1, 2])
guard_binds([-1, 2, 3])
guard_binds([-5])


def counts(items):
    """Captures used after the match are stored in the function's locals."""
    seen = 0
    for item in items:
        match item:
            case ("add", n):
                seen += n
            case ("sub", n):
                seen -= n
            case ("reset",):
                seen = 0
            case other:
                print("skipping", other)
    print("total", seen, "last n", n)


counts([("add", 5), ("sub", 2), "bad", ("add", 10)])


def walrus_guard(value):
    match value:
        case [a] if (doubled := a * 2) > 10:
            print("big", doubled)
        case _:
            print("small", doubled)


walrus_guard([7])
walrus_guard([2])


def loop_jumps(rows):
    for row in rows:
        match row:
            case ["skip", *_]:
                continue
            case ["stop"]:
                break
            case [x] if x > 100:
                print("large")
            case [name, *values] if values:
                total = 0
                for v in values:
                    if v < 0:
                        break
                    total += v
                else:
                    print(name, "sum", total)
                    continue
                print(name, "has a negative value")
    print("done")


loop_jumps([["a", 1, 2], ["skip", 5], [500], ["b", 3, -1], ["stop"], ["c", 9]])


# At module level the captures are globals.
match sorted(EXITS):
    case [first, *others]:
        print("first exit", first, "others", others)

for message in [("hello", "world"), ("bye",), 42]:
    match message:
        case (greeting, who) if greeting == "hello":
            print("greeting", who)
        case (word,):
            print("one word", word)
        case _:
            print("not a message:", message)
print(greeting, who, word)


class Config:
    match len(EXITS):
        case 3 as count:
            kind = "three"
        case _:
            kind = "other"

    match ("debug", 1):
        case (mode, level) if level > 5:
            verbose = True
        case (mode, _):
            verbose = False


print(Config.kind, Config.count, Config.mode, Config.level, Config.verbose)


def nested_functions(value):
    """A generator expression or a lambda in a guard is a nested `fn` item
    that the guard calls."""
    match value:
        case list(items) if all(isinstance(item, str) for item in items):
            return "strings"
        case [first, *_] if (lambda: first > 0)():
            return "starts positive"
        case _:
            return "other"


print(nested_functions(["a", "b"]), nested_functions([1, "a"]), nested_functions([-1]))
