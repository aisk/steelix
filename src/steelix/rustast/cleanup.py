"""Readability rewrites of the whole lowered file (design §8).

`move_returned_locals`: a local is cloned where it is returned
(`Ok(x.clone())`) only when the lowering could not see where it is
declared. Returning from the fn or closure that declares it ends the
local's life right there, so it is moved instead (`Ok(x)`). This is never
observable from Python: the object stays alive through the returned
reference either way, and no other object is dropped earlier or later.

Locals of an enclosing fn or closure are captured by reference and keep
their clone, and so do parameters (`&Bound`) and locals that a guard
borrows (`let _handling = rt::Handling::new(&err);`).
"""

from __future__ import annotations

import dataclasses
import re
from typing import Any

from steelix.rustast import nodes as rs

_RETURN_WRAPPERS = ("Ok", "Some", "rt::Flow::Return")
_PATTERN_WORDS = frozenset({"Some", "Ok", "Err", "mut", "ref", "_"})


def move_returned_locals(file: rs.File) -> rs.File:
    """`file` with returned locals moved instead of cloned."""
    return dataclasses.replace(file, items=[_item(item) for item in file.items])


def _item(item: rs.Item) -> rs.Item:
    match item:
        case rs.Fn(body=body):
            return dataclasses.replace(item, body=_body(body))
        case rs.Mod(items=items):
            return dataclasses.replace(item, items=tuple(_item(i) for i in items))
    return item


def _body(block: rs.Block) -> rs.Block:
    """The body of a fn or closure, which starts with no movable locals and
    returns its tail."""
    return _block(block, frozenset(), returns=True)


def _block(block: rs.Block, owned: frozenset[str], returns: bool = False) -> rs.Block:
    """`block` with its `return`s rewritten, and its tail too if the block
    `returns` it. Each statement sees the locals declared before it."""
    stmts = []
    for stmt in block.stmts:
        stmts.append(_node(stmt, owned))
        if isinstance(stmt, rs.Let):
            names = _pattern_names(stmt.name)
            if stmt.ty is None or stmt.ty.startswith("Bound<"):
                owned |= names
            else:
                owned -= names
            if stmt.name.startswith("_") and stmt.init is not None:
                owned -= _borrowed(stmt.init)  # a guard such as `rt::Handling`
    if block.tail is None:
        tail = None
    elif returns:
        tail = _tail(block.tail, owned)
    else:
        tail = _node(block.tail, owned)
    return rs.Block(tuple(stmts), tail)


def _node(node: Any, owned: frozenset[str]) -> Any:
    """Rewrite the `return`s in `node`, a part of a body where the locals
    `owned` (declared in the same fn or closure) are visible."""
    match node:
        case rs.Fn():
            return dataclasses.replace(node, body=_body(node.body))
        case rs.Closure(body=body):
            return dataclasses.replace(node, body=_body(body))
        case rs.Block():
            return _block(node, owned)
        case rs.Return(value) if value is not None:
            return rs.Return(_returned(_node(value, owned), owned))
        case rs.If(cond, then, orelse, pattern) if pattern is not None:
            inner = owned | _pattern_names(pattern)
            other = None if orelse is None else _node(orelse, owned)
            return rs.If(_node(cond, owned), _block(then, inner), other, pattern)
    if isinstance(node, tuple):
        return tuple(_node(item, owned) for item in node)
    if not dataclasses.is_dataclass(node) or isinstance(node, type):
        return node
    changes = {}
    for field in dataclasses.fields(node):
        value = getattr(node, field.name)
        new = _node(value, owned)
        if new is not value:
            changes[field.name] = new
    return dataclasses.replace(node, **changes) if changes else node


def _tail(expr: rs.Expr, owned: frozenset[str]) -> rs.Expr:
    """The tail expression of a fn or closure body, which it returns."""
    match expr:
        case rs.IfExpr(cond, then, orelse):
            other: rs.Block | rs.IfExpr
            if isinstance(orelse, rs.IfExpr):
                other = _tail(orelse, owned)  # type: ignore[assignment]
            else:
                other = _block(orelse, owned, returns=True)
            return rs.IfExpr(_node(cond, owned), _block(then, owned, returns=True), other)
        case rs.BlockExpr(block):
            return rs.BlockExpr(_block(block, owned, returns=True))
        case rs.Match(scrutinee, arms):
            new_arms = tuple(
                rs.Arm(
                    arm.pattern,
                    _block(arm.body, owned, returns=True),
                    None if arm.guard is None else _node(arm.guard, owned),
                )
                for arm in arms
            )
            return rs.Match(_node(scrutinee, owned), new_arms)
    return _returned(_node(expr, owned), owned)


def _returned(expr: rs.Expr, owned: frozenset[str]) -> rs.Expr:
    """`Ok(x.clone())` as `Ok(x)` if `x` is in `owned`, also inside `Some`
    and `rt::Flow::Return`, and with `.into_any()`."""
    match expr:
        case rs.Call(rs.Path(path) as func, (inner,)) if path in _RETURN_WRAPPERS:
            moved = _returned(inner, owned)
            return expr if moved is inner else rs.Call(func, (moved,))
        case rs.MethodCall(rs.Path(name) as local, "clone", ()) if name in owned:
            return local
        case rs.MethodCall(rs.MethodCall(rs.Path(name) as local, "clone", ()), "into_any", ()) if (
            name in owned
        ):
            return rs.MethodCall(local, "into_any")
    return expr


def _pattern_names(pattern: str) -> frozenset[str]:
    """The variables bound by a `let` or `if let` pattern such as `[a, mut b]`."""
    words = re.findall(r"(?:r#)?[A-Za-z_][A-Za-z_0-9]*", pattern)
    return frozenset(w for w in words if w not in _PATTERN_WORDS)


def _borrowed(expr: rs.Expr) -> frozenset[str]:
    """Locals borrowed by `expr` (`&x`), not looking into closures."""
    found: set[str] = set()

    def visit(node: Any) -> None:
        match node:
            case rs.Ref(rs.Path(name)):
                found.add(name)
            case rs.Closure() | rs.Fn():
                return
        if isinstance(node, tuple):
            for item in node:
                visit(item)
        elif dataclasses.is_dataclass(node) and not isinstance(node, type):
            for field in dataclasses.fields(node):
                visit(getattr(node, field.name))

    visit(expr)
    return frozenset(found)
