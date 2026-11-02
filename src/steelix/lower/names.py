"""Name hygiene: map Python identifiers to Rust identifiers (design §12).

The mapping is deterministic so that regenerating code gives the same names:

* Rust keywords become raw identifiers (`type` -> `r#type`); PyO3 and the
  `def!` macro strip the `r#` again where the Python name is visible.
* `self`, `Self`, `crate` and `super` can not be raw identifiers; `self`
  becomes `slf` like in PyO3 code, the others get a `_` suffix.
* Python names that collide with names the generated code uses itself (`py`,
  `globals`, ...) get a `_` suffix, repeated until the name is free.
"""

from __future__ import annotations

from collections.abc import Iterable

RUST_KEYWORDS = frozenset(
    """
    as break const continue crate else enum extern false fn for if impl in let loop
    match mod move mut pub ref return self Self static struct super trait true type
    unsafe use where while async await dyn abstract become box do final macro override
    priv typeof unsized virtual yield try gen
    """.split()
)

_NOT_RAW = {"self": "slf", "Self": "Self_", "crate": "crate_", "super": "super_", "_": "underscore"}

# Names the generated code binds or relies on inside function bodies.
FUNCTION_RESERVED = frozenset({"py", "globals", "_guard", "Ok", "Err", "Some", "None"})
# Item names the generated module defines or relies on next to the functions.
MODULE_RESERVED = frozenset({"exec_module", "Ok", "Err", "Some", "None"})


def rust_ident(name: str) -> str:
    """The Rust spelling of a Python identifier, ignoring collisions."""
    if name in _NOT_RAW:
        return _NOT_RAW[name]
    if name in RUST_KEYWORDS:
        return f"r#{name}"
    return name


class Renamer:
    """An injective mapping from the Python names of one Rust namespace."""

    def __init__(self, names: Iterable[str], reserved: Iterable[str] = ()) -> None:
        names = list(dict.fromkeys(names))
        used = set(reserved)
        self.mapping: dict[str, str] = {}
        # Names that keep their spelling win, so renames never steal them.
        for name in names:
            if rust_ident(name) == name and name not in used:
                self.mapping[name] = name
                used.add(name)
        for name in names:
            if name in self.mapping:
                continue
            candidate = rust_ident(name)
            while candidate in used:
                candidate += "_"
            self.mapping[name] = candidate
            used.add(candidate)

    def __getitem__(self, name: str) -> str:
        return self.mapping[name]

    def __contains__(self, name: str) -> bool:
        return name in self.mapping
