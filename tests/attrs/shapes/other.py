"""A module that rebinds the class name: receivers named by it are unknown."""

from shapes import Point

Point = Point


def far(p: Point):
    return p.x + p.y
