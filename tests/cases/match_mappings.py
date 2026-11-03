# Mapping patterns (design §8): JSON-style messages, `**rest`, value
# patterns as keys, and mappings that are not dicts.

import collections
import enum


class Key(enum.Enum):
    ID = "id"


def handle(message):
    match message:
        case {"type": "click", "pos": [x, y], **rest}:
            return f"click at {x},{y} extra={rest}"
        case {"type": "key", "key": str(key)} if len(key) == 1:
            return f"key {key}"
        case {"type": "key", "key": key}:
            return f"special key {key!r}"
        case {"type": "resize", "size": {"w": w, "h": h}}:
            return f"resize {w}x{h}"
        case {Key.ID: ident, **rest} if rest:
            return f"id {ident} with {sorted(rest)}"
        case {Key.ID: ident}:
            return f"just id {ident}"
        case {"type": kind, **others}:
            return f"unknown {kind} ({len(others)} more)"
        case {1: one, 2.5: _, None: nothing, b"raw": raw}:
            return f"odd keys {one} {nothing} {raw}"
        case {}:
            return "some other mapping"
        case _:
            return "not a mapping"


messages = [
    {"type": "click", "pos": [1, 2]},
    {"type": "click", "pos": (3, 4), "button": "left", "count": 2},
    {"type": "click", "pos": "xy"},
    {"type": "key", "key": "a"},
    {"type": "key", "key": "Enter"},
    {"type": "key", "key": 13},
    {"type": "resize", "size": {"w": 80, "h": 24, "unit": "chars"}},
    {Key.ID: 7, "name": "x", "kind": "y"},
    {Key.ID: 8},
    {"type": "scroll", "dy": 3},
    {1: "one", 2.5: "two", None: "none", b"raw": "raw"},
    {"other": 1},
    {},
    ["type", "click"],
    "text",
]
for message in messages:
    print(handle(message))


def counts(mapping):
    match mapping:
        case {"a": 1, **rest}:
            print("a is 1, rest", dict(sorted(rest.items())), type(rest).__name__)
        case {**everything}:
            print("everything", dict(sorted(everything.items())), everything is mapping)


counts({"a": 1, "b": 2})
counts({"a": 2, "b": 3})
counts(collections.OrderedDict(z=1, y=2))

# A `defaultdict` is looked up with `.get()`: no key is created.
d = collections.defaultdict(list)
match d:
    case {"missing": value}:
        print("found", value)
    case _:
        print("not found, keys", list(d))

# `**rest` at module level binds a global.
match {"name": "steelix", "version": 1, "license": "MIT"}:
    case {"name": name, **meta}:
        print(name, sorted(meta))
print(meta["license"])
