"""Modules in namespace packages (directories without `__init__.py`)."""

import os
import sys

import ns.tool
from ns import helper
from ns.deep import leaf
from ns.pkg import inner


def relative(path):
    return os.path.relpath(path, os.path.dirname(os.path.abspath(__file__)))


def describe(module):
    spec = module.__spec__
    print(module.__name__, module.__package__, spec.parent, relative(module.__file__))


for module in (ns.tool, helper, leaf, inner):
    describe(module)

print(ns.tool.run(3))
print(leaf.LEAF, inner.NAME)
print(type(ns.__path__).__name__, [relative(p) for p in ns.__path__])
print(ns.__spec__.origin, getattr(ns, "__file__", None))
print(sorted(name for name in vars(ns) if not name.startswith("__")))

# A regular package of the same name elsewhere on `sys.path` (here the
# standard library's `json`) wins over the namespace package `json/`.
import json

print(relative(json.__file__).endswith(os.path.join("json", "__init__.py")))
try:
    import json.shadowed
except ModuleNotFoundError as err:
    print("ModuleNotFoundError", err)

if len(sys.argv) > 1 and sys.argv[1] == "crash":
    ns.tool.crash()
