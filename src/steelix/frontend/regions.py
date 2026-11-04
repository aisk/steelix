"""Code that the lowering puts into Rust closures (design §9).

Rust has no `try` block, so code whose exceptions must be caught (the body
of a `try`, the `try` part before a `finally`, the body of a `with`) runs in
an immediately called closure whose `PyResult` is then inspected. A closure
can not initialize a variable of the enclosing fn, which matters for the
variable model (design §5): a local assigned in such a region either lives
entirely inside it (and is declared there) or must already be initialized
when the closure runs. `frontend.locals` works that out; the lowering
uses the same regions.

A `try` whose body is a single statement evaluating one expression (`x =
int(s)`, `return f(x)`, `log(x)`) needs no region: the lowering matches on
the result of that expression and assigns it afterwards.
"""

from __future__ import annotations

import ast
from collections.abc import Iterator
from dataclasses import dataclass

from steelix.frontend.desugar import AsyncFor, AsyncWith
from steelix.frontend.scope import walk_scope


@dataclass(frozen=True)
class Region:
    node: ast.Try | ast.With
    part: str | int
    """For a `try`: `"body"`, the body alone, or `"handled"`, the body with
    the handlers and `else` (what runs before `finally`). For a `with`: the
    index of the item whose closure it is."""

    def nodes(self) -> list[ast.AST]:
        """The AST nodes whose code runs inside the closure."""
        match self.node, self.part:
            case ast.Try(), "body":
                return list(self.node.body)
            case ast.Try(), "handled":
                return [*self.node.body, *self.node.handlers, *self.node.orelse]
            case ast.With(items=items, body=body), int(index):
                first = items[index]
                parts: list[ast.AST] = []
                if first.optional_vars is not None:
                    parts.append(first.optional_vars)
                for item in items[index + 1 :]:
                    parts.append(item.context_expr)
                    if item.optional_vars is not None:
                        parts.append(item.optional_vars)
                return parts + list(body)
        raise AssertionError(f"bad region {self!r}")


_SUSPENDING = (ast.Await, ast.Yield, ast.YieldFrom, AsyncFor, AsyncWith)


def suspends(nodes: list[ast.AST]) -> bool:
    """Whether code of `nodes` (not of functions defined there) can
    suspend a coroutine or generator: `await`, `yield`, `async for` or
    `async with`. In a region that does, the closure is `async` (design §8)."""
    return any(isinstance(n, _SUSPENDING) for node in nodes for n in walk_scope(node))


def expression_body(stmt: ast.Try) -> ast.expr | None:
    """The expression of a `try` whose body is one statement evaluating it:
    `x = expr` (a single name), `return expr` or `expr`. Its handlers then
    match on the result of `expr` directly, without a closure region."""
    if not stmt.handlers or len(stmt.body) != 1:
        return None
    match stmt.body[0]:
        case ast.Expr(value) | ast.Return(ast.expr() as value):
            return value
        case ast.Assign([ast.Name()], value):
            return value
    return None


def try_regions(stmt: ast.Try) -> list[Region]:
    """The closure regions of a `try` statement, outermost first."""
    regions = []
    if stmt.finalbody and stmt.handlers:
        regions.append(Region(stmt, "handled"))
    if expression_body(stmt) is None:
        regions.append(Region(stmt, "body"))
    return regions


def with_regions(stmt: ast.With) -> list[Region]:
    return [Region(stmt, i) for i in range(len(stmt.items))]


def regions_in(body: list[ast.stmt]) -> Iterator[Region]:
    """Every closure region in a function body (not in nested functions)."""
    for stmt in body:
        for node in walk_scope(stmt):
            match node:
                case ast.Try():
                    yield from try_regions(node)
                case ast.With():
                    yield from with_regions(node)


def region_node_ids(region: Region) -> set[int]:
    """`id` of every node evaluated inside the region's closure."""
    return {id(node) for part in region.nodes() for node in walk_scope(part)}
