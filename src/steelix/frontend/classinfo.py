"""What CPython's compiler works out for each class body.

`__static_attributes__` (the `self.x` attributes the methods assign) is
computed by the code generator, not by `symtable`, with rules that are
simplest to take from the compiler itself: the module is compiled once and
the value read from the code of each class body.
"""

from __future__ import annotations

import dis
import types


def static_attributes(source: str, filename: str) -> dict[tuple[str, int], tuple[str, ...]]:
    """`__static_attributes__` of every class in `source`, by the class's
    qualified name and first line (that of its first decorator)."""
    code = compile(source, filename, "exec", dont_inherit=True)
    found: dict[tuple[str, int], tuple[str, ...]] = {}
    pending = [code]
    while pending:
        current = pending.pop()
        for const in current.co_consts:
            if isinstance(const, types.CodeType):
                pending.append(const)
        value = _stored_attributes(current)
        if value is not None:
            found[(current.co_qualname, current.co_firstlineno)] = value
    return found


def _stored_attributes(code: types.CodeType) -> tuple[str, ...] | None:
    previous = None
    for instruction in dis.get_instructions(code):
        if instruction.opname == "STORE_NAME" and instruction.argval == "__static_attributes__":
            if previous is not None and isinstance(previous.argval, tuple):
                return previous.argval
            return None
        previous = instruction
    return None
