"""An error raised by a nested pattern: the traceback points at the line of
the pattern that failed, as CPython's does (design §8, §9)."""


class Point:
    __match_args__ = ("x", "y")

    def __init__(self, x, y):
        self.x = x
        self.y = y


class Line:
    __match_args__ = ("start", "end", "width")

    def __init__(self, start, end):
        self.start = start
        self.end = end


def describe(shape):
    match shape:
        case Point(x, y) if x == y:
            return "diagonal point"
        case Line(
            Point(0, 0),
            Point(x, y, 0),
        ):
            return f"from the origin to {x},{y}"
    return "other"


print(describe(Point(1, 1)))
print(describe(Line(Point(1, 0), Point(2, 2))))
print(describe(Line(Point(0, 0), Point(3, 4))))
