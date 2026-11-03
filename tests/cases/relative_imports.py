"""Relative imports and failing `from` imports in a script (design §7).

A script run as `__main__` has no package, so its relative imports fail.
"""

try:
    from . import sibling
except ImportError as err:
    print(type(err).__name__, err, err.name, err.path)

try:
    from .sibling import name
except ImportError as err:
    print(type(err).__name__, err)

try:
    from .. import parent
except ImportError as err:
    print(type(err).__name__, err)


def in_function():
    from . import sibling

    return sibling


try:
    in_function()
except ImportError as err:
    print("in a function:", err)

for module in ["sys", "json", "__main__"]:
    try:
        exec(f"from {module} import nothing")
    except ImportError as err:
        print(module, err.name, err.name_from, err.path is not None)

try:
    from __main__ import nothing
except ImportError as err:
    print(str(err).replace(__file__, "<file>"), err.name, err.name_from)

try:
    from json import nothing
except ImportError as err:
    print(err.name, err.name_from, err.path == __import__("json").__file__)
