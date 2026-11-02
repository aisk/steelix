"""What generated code needs to know about PyO3's method resolution.

Generated code calls `PyAnyMethods` on every value. A `Bound<'py, T>` of a
concrete type `T` also has the methods of `T`'s own trait (`PyDictMethods`,
..), and Rust picks those first when both have a method of the same name:
`PyDict::get_item` returns `Option` where `PyAny::get_item` raises
`KeyError`. Such a receiver has to be erased with `.as_any()` first, unless
the concrete method does exactly the same as the `PyAny` one.

The lists follow pyo3 0.30 (`src/types/*.rs`).
"""

from __future__ import annotations

# Methods every `Bound<'py, T>` of these types has besides `PyAnyMethods`,
# restricted to names that `PyAnyMethods` (or generated code) also uses.
_DICT = frozenset({"len", "is_empty", "contains", "get_item", "set_item", "del_item", "iter"})
_LIST = frozenset({"len", "is_empty", "contains", "get_item", "set_item", "del_item", "iter"})
_TUPLE = frozenset({"len", "is_empty", "contains", "get_item", "iter"})
_SET = frozenset({"len", "is_empty", "contains", "add", "iter"})
# `Bound<PyIterator>` implements `Iterator`, whose by-value methods win over
# the auto-referenced `PyAnyMethods` ones.
_ITERATOR = frozenset(
    {
        "eq",
        "ne",
        "lt",
        "le",
        "gt",
        "ge",
        "cmp",
        "partial_cmp",
        "count",
        "max",
        "min",
        "sum",
        "product",
        "map",
        "filter",
        "last",
        "nth",
        "next",
    }
)

SHADOWED: dict[str, frozenset[str]] = {
    "PyDict": _DICT,
    "PyList": _LIST,
    "PyTuple": _TUPLE,
    "PySet": _SET,
    "PyFrozenSet": _SET - {"add"},
    "PyIterator": _ITERATOR,
}

# Shadowing methods that behave exactly like the `PyAny` method on an exact
# instance of the type, which every typed value of generated code is
# (displays, `*args`, `**kwargs`, module globals). `len` returns a plain
# `usize` instead of `PyResult<usize>`.
EQUIVALENT: dict[str, frozenset[str]] = {
    "PyDict": frozenset({"len", "contains", "set_item", "del_item"}),
    "PyList": frozenset({"len", "contains"}),
    "PyTuple": frozenset({"len", "contains"}),
    "PySet": frozenset({"len", "contains"}),
    "PyFrozenSet": frozenset({"len", "contains"}),
}

INFALLIBLE_LEN = frozenset(EQUIVALENT)
"""Types whose own `len` is used for `len(x)`, without `?`."""


def must_erase(type_name: str | None, method: str) -> bool:
    """Whether calling `method` on a `Bound<'py, type_name>` would pick a
    method that does something else than the `PyAnyMethods` one."""
    if type_name is None:
        return False
    return method in SHADOWED.get(type_name, ()) and method not in EQUIVALENT.get(type_name, ())
