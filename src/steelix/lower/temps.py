"""Keep temporaries from outliving their Python statement (design §8).

A temporary of the prelude (`let value = ..;`) lives until the end of the
Rust block it is declared in: for module level code that is the end of
`exec_module`, the whole program. CPython drops the values of an expression
when the statement is done with them, which `__del__`, `weakref` callbacks
and `weakref.finalize` can observe. So after a statement is lowered:

1. A temporary whose last use is an `impl IntoPyObject` argument that is
   always evaluated (`set_item("k", &value)`) is moved into that call
   instead (`set_item("k", value)`), which drops it there.
2. If a temporary holding an object is still alive after that (it is
   borrowed, used twice, or moved on some paths only), the statement and
   its prelude go into a block of their own, `{ let value = ..; .. }`. A
   statement declaring a Python local is written `let x = { ..; value };`.

Named Rust locals of Python locals are left alone: they live until they are
reassigned or the function returns, like in CPython.
"""

from __future__ import annotations

import dataclasses
from collections.abc import Iterator
from typing import Any

from steelix.lower.rustflow import diverges
from steelix.rustast import nodes as rs


def settle(stmts: list[rs.Stmt], temps: set[str]) -> list[rs.Stmt]:
    """`stmts` of one statement (prelude included), with the temporaries
    among `temps` that they declare moved or scoped, see the module doc."""
    declared = [s.name for s in stmts if isinstance(s, rs.Let) and s.name in temps]
    if not declared:
        return stmts
    live = []
    for name in declared:
        stmts, moved = _move_last_use(stmts, name)
        if not moved:
            live.append(name)
    if not live:
        return stmts
    return _scoped(stmts, temps)


def _move_last_use(stmts: list[rs.Stmt], name: str) -> tuple[list[rs.Stmt], bool]:
    """Move the temporary `name` into its last use, if that is an
    unconditional `&name` argument in a statement of its own. Whether it is
    gone afterwards."""
    uses = [
        (index, how, conditional)
        for index, stmt in enumerate(stmts)
        for how, conditional in _uses(stmt, name, False)
    ]
    if not uses:
        return stmts, False
    index, how, conditional = uses[-1]
    if conditional:
        return stmts, False
    if how == "move":
        return stmts, True
    if how != "into" or sum(1 for i, _, _ in uses if i == index) != 1:
        return stmts, False
    target = rs.Ref(rs.Path(name), into=True)
    stmts = list(stmts)
    stmts[index] = _replace(stmts[index], target, rs.Path(name))
    return stmts, True


# Nodes whose parts may not run, or run more than once.
_CONDITIONAL = (
    rs.IfExpr,
    rs.If,
    rs.Closure,
    rs.AsyncBlock,
    rs.BlockExpr,
    rs.Match,
    rs.For,
    rs.While,
    rs.Loop,
)


def _uses(node: Any, name: str, conditional: bool) -> Iterator[tuple[str, bool]]:
    """The uses of the path `name` in `node`: how (`move`, `borrow`, or
    `into`, a borrowed `impl IntoPyObject` argument that could take the
    value), and whether it may not run or run several times."""
    if isinstance(node, rs.Fn):
        return
    if isinstance(node, rs.Binary) and node.op in ("&&", "||"):
        yield from _child_uses(node.lhs, name, conditional, "borrow")
        yield from _child_uses(node.rhs, name, True, "borrow")
        return
    match node:
        case rs.Ref(inner, _, into):
            yield from _child_uses(inner, name, conditional, "into" if into else "borrow")
            return
        case rs.MethodCall(receiver, method, args):
            # `into_any`, `unbind` and `Finally::resume` take the value, other
            # methods borrow it.
            consumes = method.startswith("into_") or method in ("unbind", "resume")
            yield from _child_uses(receiver, name, conditional, "move" if consumes else "borrow")
            for arg in args:
                yield from _child_uses(arg, name, conditional, "move")
            return
        case rs.Call(func, args):
            yield from _child_uses(func, name, conditional, "borrow")
            for arg in args:
                yield from _child_uses(arg, name, conditional, "move")
            return
        case rs.Tuple(items) | rs.Array(items):
            for item in items:
                yield from _child_uses(item, name, conditional, "move")
            return
        case rs.Let(init=init) if init is not None:
            yield from _child_uses(init, name, conditional, "move")
            return
        case rs.Assign(target, value):
            yield from _child_uses(target, name, conditional, "borrow")
            yield from _child_uses(value, name, conditional, "move")
            return
        case rs.Return(value) if value is not None:
            yield from _child_uses(value, name, conditional, "move")
            return
        case rs.Try(inner):
            yield from _child_uses(inner, name, conditional, "move")
            return
        case rs.If(cond) | rs.IfExpr(cond):
            yield from _child_uses(cond, name, conditional, "borrow")
            for child in _children(node):
                if child is not cond:
                    yield from _child_uses(child, name, True, "borrow")
            return
        case rs.Match(scrutinee):
            yield from _child_uses(scrutinee, name, conditional, "borrow")
            for child in _children(node):
                if child is not scrutinee:
                    yield from _child_uses(child, name, True, "borrow")
            return
        case rs.For(iter=iter_):
            yield from _child_uses(iter_, name, conditional, "borrow")
            for child in _children(node):
                if child is not iter_:
                    yield from _child_uses(child, name, True, "borrow")
            return
    inner_conditional = conditional or isinstance(node, _CONDITIONAL)
    for child in _children(node):
        yield from _child_uses(child, name, inner_conditional, "borrow")


def _child_uses(node: Any, name: str, conditional: bool, how: str) -> Iterator[tuple[str, bool]]:
    if node == rs.Path(name):
        yield how, conditional
    else:
        yield from _uses(node, name, conditional)


def _children(node: Any) -> Iterator[Any]:
    if isinstance(node, tuple):
        yield from node
        return
    if not dataclasses.is_dataclass(node) or isinstance(node, type):
        return
    for field in dataclasses.fields(node):
        value = getattr(node, field.name)
        if isinstance(value, tuple):
            yield from value
        elif dataclasses.is_dataclass(value):
            yield value


def _replace(node: Any, old: rs.Expr, new: rs.Expr) -> Any:
    if node == old:
        return new
    if isinstance(node, tuple):
        return tuple(_replace(item, old, new) for item in node)
    if isinstance(node, rs.Fn) or not dataclasses.is_dataclass(node) or isinstance(node, type):
        return node
    changes = {}
    for field in dataclasses.fields(node):
        value = getattr(node, field.name)
        replaced = _replace(value, old, new)
        if replaced is not value:
            changes[field.name] = replaced
    return dataclasses.replace(node, **changes) if changes else node


def _scoped(stmts: list[rs.Stmt], temps: set[str]) -> list[rs.Stmt]:
    """`stmts` in a block of their own, so that their temporaries go away at
    its end. A last statement declaring a Python local gets the block as its
    value instead: `let x = { let value = ..; f(&value)? };`."""
    if diverges(stmts):
        return stmts  # the temporaries go away with the block it leaves
    python = [
        i for i, stmt in enumerate(stmts) if isinstance(stmt, rs.Let) and stmt.name not in temps
    ]
    if not python:
        return [rs.ExprStmt(rs.BlockExpr(rs.Block(tuple(stmts))))]
    last = stmts[-1]
    if python == [len(stmts) - 1] and isinstance(last, rs.Let) and last.init is not None:
        value = rs.BlockExpr(rs.Block(tuple(stmts[:-1]), last.init))
        return [rs.Let(last.name, last.mutable, last.ty, value)]
    return stmts
