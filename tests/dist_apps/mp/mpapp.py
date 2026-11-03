"""multiprocessing with the spawn and forkserver start methods
(tests/test_dist.py): the children must run the program's transpiled
`__main__`, also when its source file is not there."""

import multiprocessing
import sys
from concurrent.futures import ProcessPoolExecutor


class Point:
    def __init__(self, x, y):
        self.x = x
        self.y = y


def square(x):
    return x * x


def norm(point):
    return point.x * point.x + point.y * point.y


def fail(x):
    if x == 3:
        raise ValueError(f"bad value {x}")
    return x


def main(method):
    ctx = multiprocessing.get_context(method)
    with ctx.Pool(2) as pool:
        print(pool.map(square, range(6)))
        print(pool.map(norm, [Point(1, 2), Point(3, 4)]))
        try:
            pool.map(fail, range(5))
        except ValueError as e:
            print("worker raised", type(e).__name__, e)
    with ProcessPoolExecutor(2, mp_context=ctx) as executor:
        print(list(executor.map(square, [7, 8])))
    print("done", method)


if __name__ == "__main__":
    main(sys.argv[1] if len(sys.argv) > 1 else "spawn")
