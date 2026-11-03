"""A small program for `steelix build` (tests/test_dist.py): a third-party
package, the standard library, a subprocess of `sys.executable`, and a
traceback through native frames."""

import json
import os
import subprocess
import sys

import greeting


def describe():
    here = os.path.dirname(__file__)
    print("script", os.path.basename(__file__), sys.argv[0] == __file__, sys.path[0] == here)
    print("argv", sys.argv[1:])
    print(greeting.hello("dist"), greeting.__name__)
    print(json.dumps({"b": [1, 2], "a": None}, sort_keys=True))
    code = "import greeting, sys; print(greeting.hello(sys.argv[1]))"
    child = subprocess.run(
        [sys.executable, "-c", code, "child"], capture_output=True, text=True, check=True
    )
    print(child.stdout.strip())


def check(value):
    if value < 0:
        raise ValueError(f"negative: {value}")
    return value


def crash():
    return check(-1)


if __name__ == "__main__":
    if sys.argv[1:] == ["crash"]:
        crash()
    describe()
