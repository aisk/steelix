"""What CPython's compiler works out for each class body and function.

`__static_attributes__` (the `self.x` attributes the methods assign) is
computed by the code generator, not by `symtable`, with rules that are
simplest to take from the compiler itself: the module is compiled once and
the value read from the code of each class body.
"""

from __future__ import annotations

import dis
import types
from dataclasses import dataclass


@dataclass(frozen=True)
class CompilerInfo:
    static_attributes: dict[tuple[str, int], tuple[str, ...]]
    """`__static_attributes__` of every class, by the class's qualified name
    and first line (that of its first decorator)."""


def compiler_info(source: str, filename: str) -> CompilerInfo:
    """What the compiler works out for the classes and functions in `source`."""
    code = compile(source, filename, "exec", dont_inherit=True)
    attributes: dict[tuple[str, int], tuple[str, ...]] = {}
    pending = [code]
    while pending:
        current = pending.pop()
        for const in current.co_consts:
            if isinstance(const, types.CodeType):
                pending.append(const)
        key = (current.co_qualname, current.co_firstlineno)
        value = _stored_attributes(current)
        if value is not None:
            attributes[key] = value
    return CompilerInfo(attributes)


def _stored_attributes(code: types.CodeType) -> tuple[str, ...] | None:
    previous = None
    for instruction in dis.get_instructions(code):
        if instruction.opname == "STORE_NAME" and instruction.argval == "__static_attributes__":
            if previous is not None and isinstance(previous.argval, tuple):
                return previous.argval
            return None
        previous = instruction
    return None
