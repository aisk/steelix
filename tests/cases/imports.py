"""Imports of standard library and third-party modules, and script metadata."""

import os
import os.path
import collections.abc as cabc
import sys
from json import dumps, loads as parse
from packaging.version import Version
import packaging.version


def local_import(text):
    import textwrap
    from math import pi as PI, sqrt
    return textwrap.shorten(text, 12), round(PI, 3), sqrt(16)


print(os.path.basename("/a/b.txt"), os.sep)
print(isinstance([], cabc.Sequence), dumps({"a": [1, 2]}), parse("[3]"))
print(Version("1.10") > Version("1.9"), packaging.version.Version("2.0"))
print(local_import("hello transpiled world"))
print(sys.argv[0] == __file__, os.path.basename(__file__), __name__)
print(sys.path[0] == os.path.dirname(os.path.realpath(__file__)))
print(len(sys.argv), __cached__)
