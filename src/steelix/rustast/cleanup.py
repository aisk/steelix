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

`declare_late`: a local declared without a value (`let kind: T;`) is
declared where it is first used, as far as the drop order allows. When that
first use is an assignment, or an `if`/`else` chain or `match` whose every
branch ends by assigning it or leaves (`return`, `break`, `continue`), it
is declared with that value: `let kind = if .. { a } else { b };`. The local
held nothing before, so evaluation order and drop timing stay the same.
"""

from __future__ import annotations

import dataclasses
import re
from collections.abc import Mapping
from typing import Any

from steelix.rustast import nodes as rs

_RETURN_WRAPPERS = ("Ok", "Some", "rt::Flow::Return")
_PATTERN_WORDS = frozenset({"Some", "Ok", "Err", "mut", "ref", "_"})
# Types of locals that Rust does not drop.
_COPY = frozenset({"bool"})


def move_returned_locals(file: rs.File) -> rs.File:
    """`file` with returned locals moved instead of cloned."""
    return dataclasses.replace(file, items=[_item(item) for item in file.items])


def declare_late(block: rs.Block, ranks: Mapping[str, int]) -> rs.Block:
    """The body of a lowered function with each declaration that holds no
    object yet (`let x: T;`, `let mut x: Option<T> = None;`) moved down to
    the statement that first uses the local, and given its value there when
    that statement assigns it. `ranks` is the position of each local in
    `co_varnames` by its Rust name.

    A declaration moves past another local's only when that one comes first
    in `co_varnames`: Rust drops locals in reverse order of declaration, like
    CPython does when the function returns (design §5). Nested fns are left
    alone, they have their own locals."""
    return _late_node(block, ranks)


def _late_node(node: Any, ranks: Mapping[str, int]) -> Any:
    """`node` with every block in it rewritten, innermost first."""
    if isinstance(node, rs.Fn | rs.ItemStmt):
        return node
    if isinstance(node, tuple):
        new = tuple(_late_node(item, ranks) for item in node)
        return node if all(a is b for a, b in zip(new, node, strict=True)) else new
    if not dataclasses.is_dataclass(node) or isinstance(node, type):
        return node
    changes = {}
    for field in dataclasses.fields(node):
        value = getattr(node, field.name)
        new = _late_node(value, ranks)
        if new is not value:
            changes[field.name] = new
    if changes:
        node = dataclasses.replace(node, **changes)
    return _late_block(node, ranks) if isinstance(node, rs.Block) else node


def _late_block(block: rs.Block, ranks: Mapping[str, int]) -> rs.Block:
    stmts = list(block.stmts)
    changed = False
    # From the last declaration up, so that one moved down is out of the way
    # of the declarations above it.
    for i in reversed(range(len(stmts))):
        changed |= _move_declaration(stmts, i, block.tail, ranks)
    return rs.Block(tuple(stmts), block.tail) if changed else block


def _move_declaration(
    stmts: list[rs.Stmt], i: int, tail: rs.Expr | None, ranks: Mapping[str, int]
) -> bool:
    """Move the declaration `stmts[i]` down if it holds nothing yet; whether
    it moved."""
    let = stmts[i]
    if not isinstance(let, rs.Let) or let.name not in ranks:
        return False
    if let.init is not None and not (
        let.init == rs.Path("None") and (let.ty or "").startswith("Option<")
    ):
        return False
    name = let.name
    rank = ranks[name]
    first = i + 1
    while first < len(stmts) and not _mentions(stmts[first], name):
        crossed = stmts[first]
        if (
            isinstance(crossed, rs.Let)
            and _COPY.isdisjoint((let.ty, crossed.ty))
            and any(ranks.get(other, -1) > rank for other in _pattern_names(crossed.name))
        ):
            break
        first += 1
    if first == len(stmts) and (tail is None or not _mentions(tail, name)):
        return False  # never used: leave it
    value = None
    if first < len(stmts) and _mentions(stmts[first], name):
        value = _assigned_value(stmts[first], name)
    if value is not None:
        later = (*stmts[first + 1 :], tail)
        # The type is inferred from the value, unless that may be a bare `None`.
        ty = let.ty if let.init is not None and _mentions(value, "None") else None
        stmts[first] = rs.Let(name, let.mutable and _reassigned(later, name), ty, value)
        del stmts[i]
        return True
    # Before the comments of the statement it moves to.
    while first > i + 1 and isinstance(stmts[first - 1], rs.Comment):
        first -= 1
    if first == i + 1:
        return False
    stmts.insert(first, let)
    del stmts[i]
    return True


def _assigned_value(stmt: rs.Stmt, name: str) -> rs.Expr | None:
    """The value `stmt` gives `name` when it ends every path through it by
    assigning `name` (or leaves) without reading or assigning it before:
    `x = v;`, an `if` with an `else` or a `match`."""
    match stmt:
        case rs.Assign(rs.Path(target), value) if target == name and not _mentions(value, name):
            return value
        case rs.If():
            return _if_value(stmt, name)
        case rs.ExprStmt(rs.Match(scrutinee, arms)) if not _mentions(scrutinee, name):
            bodies = []
            for arm in arms:
                body = _branch(arm.body, name)
                if body is None or _mentions(arm, name, body=False):
                    return None
                bodies.append(body)
            if all(body.tail is None for body in bodies):
                return None
            pairs = zip(arms, bodies, strict=True)
            return rs.Match(scrutinee, tuple(rs.Arm(a.pattern, b, a.guard) for a, b in pairs))
    return None


def _if_value(stmt: rs.If, name: str) -> rs.IfExpr | None:
    if stmt.pattern is not None or stmt.orelse is None or _mentions(stmt.cond, name):
        return None
    then = _branch(stmt.then, name)
    orelse: rs.Block | rs.IfExpr | None
    if isinstance(stmt.orelse, rs.If):
        orelse = _if_value(stmt.orelse, name)
        diverges = False
    else:
        orelse = _branch(stmt.orelse, name)
        diverges = orelse is not None and orelse.tail is None
    if then is None or orelse is None or (then.tail is None and diverges):
        return None
    return rs.IfExpr(stmt.cond, then, orelse)


def _branch(block: rs.Block, name: str) -> rs.Block | None:
    """`block` with its final assignment to `name` as the tail, or as it is
    if it always leaves; `None` if it does neither or uses `name` otherwise."""
    if block.tail is not None or not block.stmts or _mentions(block.stmts[:-1], name):
        return None
    last = block.stmts[-1]
    if isinstance(last, rs.Return | rs.Break | rs.Continue):
        return None if _mentions(last, name) else block
    value = _assigned_value(last, name)
    return None if value is None else rs.Block(block.stmts[:-1], value)


def _mentions(node: Any, name: str, body: bool = True) -> bool:
    """Whether `node` refers to the local `name` or binds a variable of that
    name (in a pattern or a parameter). Without `body`, the body of a `match`
    arm is left out."""
    match node:
        case rs.Arm(pattern, _, guard) if not body:
            return name in _pattern_names(pattern) or _mentions(guard, name)
        case rs.Path(text):
            return text == name
        case rs.Let(pattern) | rs.For(pattern) | rs.Arm(pattern) | rs.Param(pattern) if (
            name in _pattern_names(pattern)
        ):
            return True
        case rs.If(pattern=str(pattern)) | rs.While(pattern=str(pattern)) if name in _pattern_names(
            pattern
        ):
            return True
        case rs.Closure(params) if any(name in _pattern_names(p) for p in params):
            return True
    if isinstance(node, tuple):
        return any(_mentions(item, name) for item in node)
    if not dataclasses.is_dataclass(node) or isinstance(node, type):
        return False
    return any(_mentions(getattr(node, f.name), name) for f in dataclasses.fields(node))


def _reassigned(nodes: tuple[Any, ...], name: str) -> bool:
    """Whether `nodes` assign `name` or borrow it mutably."""
    found = False

    def visit(node: Any) -> None:
        nonlocal found
        match node:
            case rs.Assign(target) if _mentions(target, name):
                found = True
            case rs.Ref(rs.Path(text), mutable=True) if text == name:
                found = True
        if isinstance(node, tuple):
            for item in node:
                visit(item)
        elif dataclasses.is_dataclass(node) and not isinstance(node, type):
            for field in dataclasses.fields(node):
                visit(getattr(node, field.name))

    visit(nodes)
    return found


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
        case rs.Closure(body=body) | rs.AsyncBlock(body=body):
            return dataclasses.replace(node, body=_body(body))
        case rs.Block():
            return _block(node, owned)
        case rs.Return(value) if value is not None:
            return rs.Return(_returned(_node(value, owned), owned))
        case rs.If(cond, then, orelse, pattern) if pattern is not None:
            inner = owned | _pattern_names(pattern)
            other = None if orelse is None else _node(orelse, owned)
            return rs.If(_node(cond, owned), _block(then, inner), other, pattern)
        case rs.While(cond, body, label, pattern) if pattern is not None:
            inner = owned | _pattern_names(pattern)
            return rs.While(_node(cond, owned), _block(body, inner), label, pattern)
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
            case rs.Closure() | rs.AsyncBlock() | rs.Fn():
                return
        if isinstance(node, tuple):
            for item in node:
                visit(item)
        elif dataclasses.is_dataclass(node) and not isinstance(node, type):
            for field in dataclasses.fields(node):
                visit(getattr(node, field.name))

    visit(expr)
    return frozenset(found)
