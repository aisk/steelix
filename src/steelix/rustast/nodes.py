"""Rust AST nodes.

Only what generated code needs. Types are kept as plain text since the
generator only ever spells a handful of them.
"""

from __future__ import annotations

from dataclasses import dataclass, field

# --- expressions ---------------------------------------------------------------


class Expr:
    """Base class for expressions."""


@dataclass(frozen=True)
class Path(Expr):
    """A path or plain identifier: `n`, `rt::int`, `pyo3::pyclass::CompareOp::Lt`."""

    text: str


@dataclass(frozen=True)
class IntLit(Expr):
    value: int
    suffix: str = ""


@dataclass(frozen=True)
class FloatLit(Expr):
    value: float


@dataclass(frozen=True)
class StrLit(Expr):
    value: str


@dataclass(frozen=True)
class RawStrLit(Expr):
    """A raw string literal `r#"..."#`, used for Python source kept as is.
    The value must not contain a carriage return (rustc rejects a bare CR)."""

    value: str


@dataclass(frozen=True)
class ByteStrLit(Expr):
    value: bytes


@dataclass(frozen=True)
class BoolLit(Expr):
    value: bool


@dataclass(frozen=True)
class Call(Expr):
    func: Expr
    args: tuple[Expr, ...] = ()


@dataclass(frozen=True)
class MethodCall(Expr):
    receiver: Expr
    method: str
    args: tuple[Expr, ...] = ()


@dataclass(frozen=True)
class MacroCall(Expr):
    """`path!(args)`. Arguments are expressions or `Tokens`."""

    path: str
    args: tuple[Expr, ...] = ()


@dataclass(frozen=True)
class Try(Expr):
    """The `?` operator."""

    expr: Expr


@dataclass(frozen=True)
class Ref(Expr):
    """`&expr`, or `&mut expr`."""

    expr: Expr
    mutable: bool = False


@dataclass(frozen=True)
class Tuple(Expr):
    elems: tuple[Expr, ...]


@dataclass(frozen=True)
class Array(Expr):
    elems: tuple[Expr, ...]


@dataclass(frozen=True)
class Unary(Expr):
    """A prefix operator: `!x`."""

    op: str
    expr: Expr


@dataclass(frozen=True)
class Binary(Expr):
    """A binary operator: `&&`, `||` or a comparison such as `!=`."""

    op: str
    lhs: Expr
    rhs: Expr


@dataclass(frozen=True)
class BlockExpr(Expr):
    """`{ stmts; tail }` used as an expression."""

    block: Block


@dataclass(frozen=True)
class IfExpr(Expr):
    """`if cond { .. } else { .. }` used as an expression."""

    cond: Expr
    then: Block
    orelse: Block | IfExpr


@dataclass(frozen=True)
class Tokens(Expr):
    """A macro argument that is not an expression, such as `name = value` or
    a signature: verbatim token text interleaved with expressions.

    The text must not refer to locals or items; such references are
    expression parts, so that the verifier sees them.
    """

    parts: tuple[str | Expr, ...]


# --- statements ----------------------------------------------------------------


class Stmt:
    """Base class for statements."""


@dataclass(frozen=True)
class Let(Stmt):
    name: str
    """A variable name or a pattern such as `[a, b]`."""
    mutable: bool = False
    ty: str | None = None
    init: Expr | None = None


@dataclass(frozen=True)
class Assign(Stmt):
    target: Expr
    value: Expr


@dataclass(frozen=True)
class ExprStmt(Stmt):
    expr: Expr


@dataclass(frozen=True)
class Return(Stmt):
    value: Expr | None = None


@dataclass(frozen=True)
class If(Stmt):
    cond: Expr
    then: Block
    orelse: Block | If | None = None


@dataclass(frozen=True)
class While(Stmt):
    cond: Expr
    body: Block
    label: str | None = None


@dataclass(frozen=True)
class Loop(Stmt):
    body: Block
    label: str | None = None


@dataclass(frozen=True)
class For(Stmt):
    pat: str
    iter: Expr
    body: Block
    label: str | None = None


@dataclass(frozen=True)
class Break(Stmt):
    label: str | None = None


@dataclass(frozen=True)
class Continue(Stmt):
    label: str | None = None


@dataclass(frozen=True)
class LabeledBlock(Stmt):
    """`'label: { .. }`, left early with `break 'label`."""

    label: str
    body: Block


@dataclass(frozen=True)
class Comment(Stmt):
    text: str


@dataclass(frozen=True)
class ItemStmt(Stmt):
    """An item declared inside a block, such as a nested `fn`."""

    item: Item


@dataclass(frozen=True)
class Block:
    stmts: tuple[Stmt, ...] = ()
    tail: Expr | None = None


# --- items ---------------------------------------------------------------------


class Item:
    """Base class for module level items."""


@dataclass(frozen=True)
class Use(Item):
    path: str


@dataclass(frozen=True)
class Param:
    name: str
    ty: str


@dataclass(frozen=True)
class Fn(Item):
    name: str
    params: tuple[Param, ...]
    ret: str
    body: Block
    lifetimes: tuple[str, ...] = ()
    public: bool = True
    attrs: tuple[str, ...] = ()
    doc: tuple[str, ...] = ()
    comments: tuple[str, ...] = ()


@dataclass(frozen=True)
class ModDecl(Item):
    """`mod name;`"""

    name: str
    public: bool = False


@dataclass
class File:
    items: list[Item] = field(default_factory=list)
    inner_doc: tuple[str, ...] = ()
    inner_attrs: tuple[str, ...] = ()
