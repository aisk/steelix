"""Function locals that are a typed Rust local of a Python container
(design §5): a `Bound<'py, PyList>`, `PyDict` or `PySet` instead of a
`Bound<'py, PyAny>`.

A local qualifies when every value it is ever assigned is, by the
language's own rules, an exact `list`, an exact `dict` or an exact `set`
(never a subclass), the same one each time:

- a list, dict or set display (`[]`, `[a, *b]`, `{}`, `{"k": v, **d}`, `{a}`)
- a list, dict or set comprehension
- another such local of the same kind

Calls such as `list()` or `dict(x)` do not: the builtin is looked up when
the call runs and may have been replaced. Conditional expressions and
`and`/`or` are left out as well, their lowering yields an object.

The local must also be an ordinary Rust local, with the same rules as a
`bool` local (`frontend.booleans.plain_locals`): assigned only by plain
`x = value` statements, never read while it may be unassigned, not a
parameter and not captured by a nested function.

Typing a local is for the reader, so a local also stays an object when the
type would get in the way: `xs[i]`, `d[k]`, `xs[i] = v` and `del xs[i]` are
`PyAnyMethods` calls that PyO3's own `get_item`, .. of the concrete type
shadow with other semantics (`rustast.pyo3`), and would have to be written
`xs.as_any().get_item(i)?`. (`d[k] = v` and `del d[k]` are the same call on
a `PyDict`.)
"""

from __future__ import annotations

import ast

from steelix.frontend.booleans import plain_locals
from steelix.frontend.locals import LocalsInfo
from steelix.frontend.scope import FunctionScope, scope_body, walk_scope
from steelix.rustast.pyo3 import must_erase

_DISPLAYS: dict[type[ast.expr], str] = {
    ast.List: "PyList",
    ast.ListComp: "PyList",
    ast.Dict: "PyDict",
    ast.DictComp: "PyDict",
    ast.Set: "PySet",
    ast.SetComp: "PySet",
}


def container_locals(
    fn: ast.FunctionDef | ast.Lambda | ast.GeneratorExp, scope: FunctionScope, info: LocalsInfo
) -> dict[str, str]:
    """The locals of `fn` that are a typed container, with the PyO3 type
    of each (`PyList`, `PyDict` or `PySet`)."""
    if not isinstance(fn, ast.FunctionDef):
        return {}
    assigned = plain_locals(fn, scope, info)
    # What each local would be: the type of a display assigned to it, else
    # that of a local assigned to it.
    kinds: dict[str, str] = {}
    for name, values in assigned.items():
        for value in values:
            if type(value) in _DISPLAYS:
                kinds[name] = _DISPLAYS[type(value)]
                break
    changed = True
    while changed:
        changed = False
        for name, values in assigned.items():
            if name in kinds:
                continue
            for value in values:
                if isinstance(value, ast.Name) and value.id in kinds:
                    kinds[name] = kinds[value.id]
                    changed = True
                    break
    # Drop the locals that are assigned anything else until nothing changes.
    changed = True
    while changed:
        changed = False
        for name in sorted(kinds):
            if not all(_kind(value, kinds) == kinds[name] for value in assigned[name]):
                del kinds[name]
                changed = True
    return {name: kind for name, kind in kinds.items() if name not in _subscripted(fn, kinds)}


_SUBSCRIPT = {ast.Load: "get_item", ast.Store: "set_item", ast.Del: "del_item"}


def _subscripted(fn: ast.FunctionDef, kinds: dict[str, str]) -> set[str]:
    """The locals that are subscripted in a way that needs the type erased."""
    found: set[str] = set()
    for part in scope_body(fn):
        for node in walk_scope(part):
            match node:
                case ast.Subscript(ast.Name(name), ctx=ctx) if name in kinds:
                    if must_erase(kinds[name], _SUBSCRIPT[type(ctx)]):
                        found.add(name)
                case ast.AugAssign(ast.Subscript(ast.Name(name))) if name in kinds:
                    found.add(name)  # reads the item first
    return found


def _kind(expr: ast.expr, kinds: dict[str, str]) -> str | None:
    if isinstance(expr, ast.Name):
        return kinds.get(expr.id)
    return _DISPLAYS.get(type(expr))
