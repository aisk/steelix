"""Decide how a reference to a global name is compiled (design §7)."""

from __future__ import annotations

import ast
import builtins
from dataclasses import dataclass
from enum import Enum

from steelix.frontend.bindings import ModuleBindings
from steelix.frontend.desugar import suspends
from steelix.frontend.scope import function_kind

_BUILTIN_NAMES = frozenset(dir(builtins))


@dataclass(frozen=True)
class Slot:
    """What a direct call passes for one parameter of the callee."""

    kind: str
    """`arg` (a positional argument), `keyword`, `default` (read from the
    function object), `varargs` (a tuple of positional arguments) or `varkw`
    (a dict of keyword arguments)."""
    indices: tuple[int, ...] = ()
    """Positions in `call.args` (`arg`, `varargs`) or `call.keywords`
    (`keyword`, `varkw`)."""


@dataclass(frozen=True)
class Direct:
    """A call that goes straight to the Rust function of a module level `def`."""

    fn: ast.FunctionDef
    slots: tuple[Slot, ...] = ()
    """One per parameter of `fn`, in source order."""


class Lookup(Enum):
    GLOBAL = "global"
    """Look the name up in the module globals at run time, then in builtins."""
    BUILTIN = "builtin"
    """The name is not bound by the module, so it can only be a builtin."""


Resolution = Direct | Lookup


def resolve_global(bindings: ModuleBindings, name: str) -> Lookup:
    """Resolve a global `name` that is not called directly."""
    if not bindings.dynamic and not bindings.is_module_bound(name) and name in _BUILTIN_NAMES:
        return Lookup.BUILTIN
    return Lookup.GLOBAL


def resolve_call(
    bindings: ModuleBindings, name: str, call: ast.Call, module_index: int | None = None
) -> Resolution:
    """Resolve the global `name` called by `call`.

    `module_index` is the index of the enclosing top-level statement when the
    call is in module level code, where a direct call is only allowed once the
    `def` has executed.

    A coroutine is only called directly by `await f(x)` and a generator by
    `yield from g(x)`, which run the Rust future in place. Any other call
    creates the coroutine or generator object, through the function object.
    """
    fn = bindings.direct_function(name)
    if fn is not None and _CALLED_BY.get(function_kind(fn), suspends(call)) != suspends(call):
        fn = None  # a coroutine or generator, not called by `await` or `yield from`
    if fn is not None and (module_index is None or bindings.def_index(name) < module_index):
        slots = bind_static(fn.args, call)
        if slots is not None:
            return Direct(fn, slots)
    return resolve_global(bindings, name)


_CALLED_BY = {"coroutine": "await", "generator": "yield from"}


def bind_static(args: ast.arguments, call: ast.Call) -> tuple[Slot, ...] | None:
    """Bind the arguments of `call` to the parameters `args` at transpile time.

    Returns `None` when that is not possible: argument unpacking, or a call
    that raises `TypeError`, which is left to the function object so that the
    error is exactly CPython's.
    """
    if any(isinstance(a, ast.Starred) for a in call.args):
        return None
    if any(k.arg is None for k in call.keywords):
        return None
    positional = [a.arg for a in args.posonlyargs + args.args]
    first_default = len(positional) - len(args.defaults)
    slots: list[Slot | None] = [None] * len(positional)
    for i in range(min(len(call.args), len(positional))):
        slots[i] = Slot("arg", (i,))
    extra = tuple(range(len(positional), len(call.args)))
    if extra and args.vararg is None:
        return None
    kwonly = [a.arg for a in args.kwonlyargs]
    kwonly_slots: list[Slot | None] = [None] * len(kwonly)
    varkw: list[int] = []
    posonly = len(args.posonlyargs)
    for j, keyword in enumerate(call.keywords):
        name = keyword.arg
        if name in positional[posonly:]:
            index = positional.index(name)
            if slots[index] is not None:
                return None
            slots[index] = Slot("keyword", (j,))
        elif name in kwonly:
            kwonly_slots[kwonly.index(name)] = Slot("keyword", (j,))
        elif args.kwarg is not None:
            varkw.append(j)
        else:
            return None
    for i, slot in enumerate(slots):
        if slot is None:
            if i < first_default:
                return None
            slots[i] = Slot("default")
    for i, slot in enumerate(kwonly_slots):
        if slot is None:
            if args.kw_defaults[i] is None:
                return None
            kwonly_slots[i] = Slot("default")
    result = [s for s in slots if s is not None]
    if args.vararg is not None:
        result.append(Slot("varargs", extra))
    result += [s for s in kwonly_slots if s is not None]
    if args.kwarg is not None:
        result.append(Slot("varkw", tuple(varkw)))
    return tuple(result)
