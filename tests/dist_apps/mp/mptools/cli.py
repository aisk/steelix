"""`python -m mptools.cli`: a pool started from a module run with `-m`,
with functions from the module itself and from another submodule."""

import multiprocessing
import sys

from mptools.work import cube, where


def double(x):
    return (2 * x, __name__)


def main(method):
    ctx = multiprocessing.get_context(method)
    with ctx.Pool(2) as pool:
        print(pool.map(cube, range(4)))
        print(pool.map(where, [1]))
        print(pool.map(double, [1, 2]))
    print("done", method)


if __name__ == "__main__":
    main(sys.argv[1] if len(sys.argv) > 1 else "spawn")
