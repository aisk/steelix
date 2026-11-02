"""Generic rewrites of a fn body after lowering.

They see the body of one fn: nested fn items have their own parameters and
are left alone. Text in `Tokens` is not looked at, only their expression parts
(see `rustast.nodes.Tokens`).
"""

from __future__ import annotations

import dataclasses
from collections.abc import Callable
from typing import Any

from steelix.rustast import nodes as rs


def count_path(block: rs.Block, name: str) -> int:
    """How often the plain path `name` occurs in `block`."""
    count = 0

    def visit(node: rs.Expr) -> rs.Expr:
        nonlocal count
        if node == rs.Path(name):
            count += 1
        return node

    map_exprs(block, visit)
    return count


def replace_path(block: rs.Block, name: str, replacement: rs.Expr) -> rs.Block:
    """`block` with every plain path `name` replaced."""
    target = rs.Path(name)
    return map_exprs(block, lambda node: replacement if node == target else node)


def map_exprs[T](node: T, fn: Callable[[rs.Expr], rs.Expr]) -> T:
    """Rebuild `node` bottom-up, applying `fn` to every expression."""
    return _map(node, fn)


def _map(node: Any, fn: Callable[[rs.Expr], rs.Expr]) -> Any:
    if isinstance(node, tuple):
        return tuple(_map(item, fn) for item in node)
    if isinstance(node, rs.Fn):
        return node  # a nested fn item
    if not dataclasses.is_dataclass(node) or isinstance(node, type):
        return node
    changes = {}
    for f in dataclasses.fields(node):
        value = getattr(node, f.name)
        mapped = _map(value, fn)
        if mapped is not value:
            changes[f.name] = mapped
    if changes:
        node = dataclasses.replace(node, **changes)
    if isinstance(node, rs.Expr):
        node = fn(node)
    return node
