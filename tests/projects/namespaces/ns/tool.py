from . import helper
from .deep.leaf import LEAF


def run(n):
    return helper.double(n) + len(LEAF)


def crash():
    helper.fail("from the namespace package")
