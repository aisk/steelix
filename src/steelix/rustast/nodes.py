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
    """`path!(args)`. Arguments are expressions or `Raw` token text."""

    path: str
    args: tuple[Expr, ...] = ()


@dataclass(frozen=True)
class Try(Expr):
    """The `?` operator."""

    expr: Expr


@dataclass(frozen=True)
class Ref(Expr):
    """`&expr`."""

    expr: Expr


@dataclass(frozen=True)
class Tuple(Expr):
    elems: tuple[Expr, ...]


@dataclass(frozen=True)
class Raw(Expr):
    """Verbatim tokens, used for macro arguments such as `name = "x"`."""

    text: str


# --- statements ----------------------------------------------------------------


class Stmt:
    """Base class for statements."""


@dataclass(frozen=True)
class Let(Stmt):
    name: str
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


@dataclass(frozen=True)
class Loop(Stmt):
    body: Block


@dataclass(frozen=True)
class Comment(Stmt):
    text: str


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
