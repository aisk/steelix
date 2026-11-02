"""Decide how a reference to a global name is compiled (design §7)."""

from __future__ import annotations

import ast
import builtins
from dataclasses import dataclass
from enum import Enum

from steelix.frontend.bindings import ModuleBindings

_BUILTIN_NAMES = frozenset(dir(builtins))


@dataclass(frozen=True)
class Direct:
    """A call that goes straight to the Rust function of a module level `def`."""

    fn: ast.FunctionDef


class Lookup(Enum):
    GLOBAL = "global"
    """Look the name up in the module globals at run time, then in builtins."""
    BUILTIN = "builtin"
    """The name is not bound by the module, so it can only be a builtin."""


Resolution = Direct | Lookup


def resolve_global(
    bindings: ModuleBindings,
    name: str,
    call_arity: int | None = None,
    module_index: int | None = None,
) -> Resolution:
    """Resolve a global `name`.

    `call_arity` is the number of positional arguments when `name` is called.
    `module_index` is the index of the enclosing top-level statement when the
    reference is in module level code, where a direct call is only allowed once
    the `def` has executed.
    """
    if call_arity is not None:
        fn = bindings.direct_function(name)
        if (
            fn is not None
            and call_arity == len(fn.args.posonlyargs) + len(fn.args.args)
            and (module_index is None or bindings.def_index(name) < module_index)
        ):
            return Direct(fn)
    if not bindings.dynamic and not bindings.is_module_bound(name) and name in _BUILTIN_NAMES:
        return Lookup.BUILTIN
    return Lookup.GLOBAL
