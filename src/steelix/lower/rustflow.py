"""Questions about lowered Rust code that the statement lowering asks.

* Whether a block can fall off its end (`diverges`), which decides where a
  closure needs its final `Ok(..)` and where a line marker may follow. It
  has to agree with rustc, which warns about unreachable code.
* Whether code can raise (`fallible`), which decides where the traceback
  line has to be up to date (design §9).
"""

from __future__ import annotations

import ast
import dataclasses
from collections.abc import Iterator
from typing import Any

from steelix.rustast import nodes as rs


def diverges(stmts: tuple[rs.Stmt, ...] | list[rs.Stmt]) -> bool:
    """Whether control never reaches the end of `stmts` (as rustc sees it)."""
    return bool(stmts) and _diverges(stmts[-1])


def _diverges(stmt: rs.Stmt) -> bool:
    match stmt:
        case rs.Return() | rs.Break() | rs.Continue():
            return True
        case rs.ExprStmt(rs.MacroCall("unreachable" | "panic")):
            return True
        case rs.If(_, then, orelse):
            if orelse is None:
                return False
            other = _diverges(orelse) if isinstance(orelse, rs.If) else diverges(orelse.stmts)
            return diverges(then.stmts) and other
        case rs.ExprStmt(rs.Match(_, arms)):
            return all(diverges(arm.body.stmts) and arm.body.tail is None for arm in arms)
        case rs.Loop(body, label):
            return not _breaks_out(body, label)
        case rs.LabeledBlock(label, body):
            return diverges(body.stmts) and not _breaks_out(body, label, loop=False)
    return False


def returns_as_tail(
    stmts: tuple[rs.Stmt, ...] | list[rs.Stmt],
) -> tuple[list[rs.Stmt], rs.Expr | None]:
    """The statements of a fn or closure body without a final `return`,
    and the tail expression it becomes: `return Ok(x);` as `Ok(x)`, and a
    final `match` or `if` whose branches all end in `return` as a `match` or
    `if` expression with those values (clippy's `needless_return`)."""
    if not stmts:
        return list(stmts), None
    tail = _as_tail(stmts[-1])
    if tail is None:
        return list(stmts), None
    return list(stmts[:-1]), tail


def _as_tail(stmt: rs.Stmt) -> rs.Expr | None:
    match stmt:
        case rs.Return(value) if value is not None:
            return value
        case rs.ExprStmt(rs.Match(scrutinee, arms)) if _diverges(stmt):
            return rs.Match(scrutinee, tuple(_tail_arm(arm) for arm in arms))
        case rs.If(cond, then, rs.Block() | rs.If() as orelse, None) if _diverges(stmt):
            if isinstance(orelse, rs.If):
                other = _as_tail(orelse)
                if not isinstance(other, rs.IfExpr):
                    return None
                return rs.IfExpr(cond, _tail_block(then), other)
            return rs.IfExpr(cond, _tail_block(then), _tail_block(orelse))
    return None


def _tail_block(block: rs.Block) -> rs.Block:
    stmts, tail = returns_as_tail(block.stmts)
    return block if tail is None else rs.Block(tuple(stmts), tail)


def _tail_arm(arm: rs.Arm) -> rs.Arm:
    return rs.Arm(arm.pattern, _tail_block(arm.body), arm.guard)


def _breaks_out(body: rs.Block, label: str | None, loop: bool = True) -> bool:
    """Whether `body` of a loop (or labeled block) contains a `break` that
    leaves it."""

    def visit(node: Any, depth: int) -> bool:
        match node:
            case rs.Break(target):
                return target == label if target is not None else (loop and depth == 0)
            case rs.Loop() | rs.While() | rs.For():
                return any(visit(child, depth + 1) for child in children(node))
            case rs.Closure() | rs.Fn():
                return False
        return any(visit(child, depth) for child in children(node))

    return any(visit(stmt, 0) for stmt in body.stmts)


def children(node: Any) -> Iterator[Any]:
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


def walk(node: Any) -> Iterator[Any]:
    """`node` and everything in it, not looking into nested fn items or
    line markers (`frame.line(n)`)."""
    yield node
    for child in children(node):
        if isinstance(child, rs.Fn) or _is_marker(child):
            continue
        yield from walk(child)


def _is_marker(node: Any) -> bool:
    return (
        isinstance(node, rs.MethodCall)
        and node.receiver == rs.Path("frame")
        and (node.method == "line")
    )


def fallible(stmts: list[rs.Stmt] | tuple[rs.Stmt, ...]) -> bool:
    """Whether running `stmts` may raise a Python exception (or runs Python
    code, which may look at the traceback): a `?`, a `return` of a
    `PyResult` that is not `Ok(..)`, or anything done with the frame."""
    for stmt in stmts:
        if isinstance(stmt, rs.ExprStmt) and _is_marker(stmt.expr):
            continue
        for node in walk(stmt):
            match node:
                case rs.Try() | rs.Path("frame"):
                    return True
                case rs.Return(value) if value is not None and not _is_ok(value):
                    return True
    return False


def _is_ok(expr: rs.Expr) -> bool:
    return isinstance(expr, rs.Call) and expr.func == rs.Path("Ok")


def statement_line(stmt: ast.stmt) -> int:
    """The traceback line of a simple statement that raises.

    CPython reports the line of the instruction that raised. Native code
    knows the statement only: its first line, or for `return (` / `x = (`
    with the value on the next lines, the first line of the value.
    """
    match stmt:
        case ast.Return(ast.expr() as value) | ast.Assign(value=value) | ast.Expr(value):
            return value.lineno
    return stmt.lineno
