"""Drives the `json` package next to it, a copy of the standard library's.

The copy shadows the standard library's `json` (it is in `sys.path[0]`), so
the transpiled program runs the package's transpiled modules. The package
picks the C accelerator `_json` at import time where it can; the pure-Python
scanner, string functions and encoder are exercised explicitly as well.

    python app.py            exercises the package
    python app.py tool ARGS  runs `json.tool.main()` with ARGS
    python app.py crash      ends with an uncaught `JSONDecodeError`
"""

import sys
from collections import OrderedDict
from decimal import Decimal

import json
from json import decoder, encoder, scanner

DOC = '{"b": [1, 2.5, "caf\\u00e9", null, true], "a": {"nested": "\\ud83d\\ude00"}, "n": -0.0}'


def accelerated():
    """Which functions came from `_json`."""
    return {
        "scanner": scanner.make_scanner is scanner.c_make_scanner,
        "scanstring": decoder.scanstring is decoder.c_scanstring,
        "encode_basestring_ascii": (
            encoder.encode_basestring_ascii is encoder.c_encode_basestring_ascii
        ),
        "make_encoder": encoder.c_make_encoder is not None,
    }


def roundtrips():
    value = json.loads(DOC)
    print(value)
    print(json.dumps(value))
    print(json.dumps(value, sort_keys=True, ensure_ascii=False))
    # `indent` uses the pure-Python encoder `_make_iterencode`.
    print(json.dumps(value, indent=2, sort_keys=True, separators=(",", ": ")))
    print(json.dumps([1, {"x": (2, 3)}], indent="\t"))
    print(json.loads(DOC, object_pairs_hook=OrderedDict))
    print(json.loads("[1.10, 2]", parse_float=Decimal, parse_int=str))
    print(json.loads('{"k": NaN, "j": -Infinity}'))
    print(json.dumps(float("inf")), json.dumps({1: 2, None: 3, 2.5: 4, True: 5}))
    print(json.dumps({"z": 1, "y": [1, 2]}, sort_keys=True, indent=1))


def pure_python():
    """The fallbacks the package uses without `_json`."""
    dec = json.JSONDecoder()
    dec.parse_string = decoder.py_scanstring
    dec.scan_once = scanner.py_make_scanner(dec)
    print(dec.decode(DOC))
    print(decoder.py_scanstring('"abc\\n\\u00e9" tail', 1))
    print(encoder.py_encode_basestring_ascii("café \U0001f600 \n"))
    print(encoder.py_encode_basestring("café \n"))
    enc = json.JSONEncoder(sort_keys=True)
    print("".join(enc.iterencode({"b": [1, 2], "a": None}, _one_shot=False)))
    try:
        dec.decode('{"a": [1, 2,]}')
    except json.JSONDecodeError as err:
        print("pure:", err, err.pos, err.lineno, err.colno)


class Point:
    def __init__(self, x, y):
        self.x = x
        self.y = y


class PointEncoder(json.JSONEncoder):
    def default(self, o):
        if isinstance(o, Point):
            return {"x": o.x, "y": o.y}
        return super().default(o)


def errors():
    for text in ['{"a": }', "[1, 2", '{"a" 1}', '"\\x"', "", "[1] x"]:
        try:
            json.loads(text)
        except json.JSONDecodeError as err:
            print(repr(text), "->", err.msg, err.pos, err.lineno, err.colno)
    try:
        json.dumps({"p": object()})
    except TypeError as err:
        print("TypeError:", err, err.__notes__)
    try:
        json.dumps([float("nan")], allow_nan=False)
    except ValueError as err:
        print("ValueError:", err)
    circular = []
    circular.append(circular)
    try:
        json.dumps(circular)
    except ValueError as err:
        print("ValueError:", err)
    print(json.dumps([Point(1, 2)], cls=PointEncoder))
    print(json.dumps(Point(3, 4), default=vars, indent=0))


def main():
    print(accelerated())
    roundtrips()
    pure_python()
    errors()
    print(json.decoder.__name__, json.decoder.__package__, json.scanner.__spec__.name)


if __name__ == "__main__":
    if sys.argv[1:2] == ["tool"]:
        import json.tool

        sys.argv[1:2] = []
        json.tool.main()
    elif sys.argv[1:2] == ["crash"]:
        json.loads('{"ok": [1, 2, ]}')
    else:
        main()
