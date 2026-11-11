"""How a lowered expression is spelled in Rust (`Value`), shared by the
parts of the expression lowering (`lower.expr`, `lower.calls`,
`lower.defs`) and the statement lowering."""

from __future__ import annotations

import ast
from collections.abc import Callable
from dataclasses import dataclass
from enum import Enum

from steelix.rustast import nodes as rs
from steelix.rustast.pyo3 import must_erase


class Kind(Enum):
    OWNED = "owned"
    """A `Bound<'py, PyAny>` value, usually a call ending in `?`."""
    REF = "ref"
    """A `&Bound<'py, PyAny>`: a parameter or a checked `Option` local."""
    PLACE = "place"
    """A local variable (or temporary) of type `Bound<'py, PyAny>`."""
    LITERAL = "literal"
    """A Rust literal usable as `impl IntoPyObject`; `owned` builds the object."""
    BOOL = "bool"
    """A Rust `bool` standing for a Python `bool`, e.g. the result of `in`."""
    NATIVE = "native"
    """Another Rust value standing for a Python object, e.g. the `usize` of `len`."""


@dataclass(frozen=True)
class Value:
    expr: rs.Expr
    kind: Kind
    owned_literal: rs.Expr | None = None
    """For `LITERAL`, `BOOL` and `NATIVE`: an expression building the object."""
    typed: str | None = None
    """The concrete type `T` if the value is a `Bound<'py, T>` such as a
    `PyString`, not a `Bound<'py, PyAny>`."""
    pure: bool | None = None
    """Evaluating it has no effect and can not fail. Derived from the kind
    when not given."""
    base: str | None = None
    """Name for a temporary holding the value, if the expression suggests one."""
    temp: bool = False
    """A temporary of the generated code, which its single owned use may move."""
    caught: bool = False
    """A `PLACE` that is the `rt::Caught` of an `except` block
    (`let e = frame.catch(err);`): used like a `Bound`, which it
    dereferences to, but returning it clones the `Bound`."""
    awaited: bool = False
    """The result of a native coroutine or generator awaited in place,
    `rt::delegate(f(m, &x)).await?` for `await f(x)` or `yield from f(x)`."""

    @property
    def is_pure(self) -> bool:
        if self.pure is not None:
            return self.pure
        match self.kind:
            case Kind.LITERAL | Kind.PLACE:
                return True
            case Kind.REF:
                return isinstance(self.expr, rs.Path)
        return False

    # --- spellings for the different use sites ---

    def arg(self) -> rs.Expr:
        """As an `impl IntoPyObject` argument of a PyO3 method or call tuple.
        A temporary is borrowed here, and moved instead if this is its only
        use (`lower.temps`)."""
        if self.kind is Kind.PLACE:
            return rs.Ref(self.expr, into=self.temp)
        return self.expr

    def ref(self) -> rs.Expr:
        """As an argument of a function whose parameter is declared as a
        `&Bound<'py, PyAny>`: a generated function or a runtime helper. A
        `&Bound<'py, PyList>` is coerced there (`Bound<T>` dereferences to
        `Bound<PyAny>`)."""
        match self.kind:
            case Kind.REF:
                return self.expr
            case Kind.LITERAL | Kind.BOOL | Kind.NATIVE:
                assert self.owned_literal is not None
                return rs.Ref(self.owned_literal)
        if self.typed and self.kind is Kind.OWNED:
            # `&PyList::new(..)?` does not coerce: `?` needs the exact type.
            return rs.MethodCall(self.expr, "as_any")
        return rs.Ref(self.expr)

    def erased(self) -> rs.Expr:
        """As a `&Bound<'py, PyAny>` of exactly that type, where nothing
        coerces: an array element, a generic argument."""
        if self.typed and self.kind in (Kind.REF, Kind.PLACE):
            return rs.MethodCall(self.expr, "as_any")
        return self.ref()

    def method(self, name: str, *args: rs.Expr) -> rs.MethodCall:
        """`value.name(args)`, calling the `PyAnyMethods` method `name`.

        A concrete type such as `PyDict` has its own `get_item`, `len`, ..
        that Rust would pick instead (see `rustast.pyo3`), so such a receiver
        is erased to `PyAny` first where that makes a difference.
        """
        if self.kind in (Kind.LITERAL, Kind.BOOL, Kind.NATIVE):
            assert self.owned_literal is not None
            receiver = self.owned_literal
        elif must_erase(self.typed, name):
            receiver = rs.MethodCall(self.expr, "as_any")
        else:
            receiver = self.expr
        return rs.MethodCall(receiver, name, args)

    def bound(self) -> rs.Expr:
        """As an expression `&expr` makes a `&Bound<'py, PyAny>`-like reference of."""
        if self.kind in (Kind.LITERAL, Kind.BOOL, Kind.NATIVE):
            assert self.owned_literal is not None
            return self.owned_literal
        return self.expr

    def identity(self) -> rs.Expr:
        """As the argument of `Bound::is`, which takes `AsRef<Py<PyAny>>`."""
        if self.kind in (Kind.LITERAL, Kind.BOOL, Kind.NATIVE):
            assert self.owned_literal is not None
            return self.owned_literal
        return self.arg()

    def owned(self) -> rs.Expr:
        """As an owned `Bound<'py, PyAny>`."""
        match self.kind:
            case Kind.LITERAL | Kind.BOOL | Kind.NATIVE:
                assert self.owned_literal is not None
                return self.owned_literal
            case Kind.PLACE if self.temp:
                expr: rs.Expr = self.expr
            case Kind.REF | Kind.PLACE:
                expr = rs.MethodCall(self.expr, "clone")
            case _:
                expr = self.expr
        return rs.MethodCall(expr, "into_any") if self.typed else expr

    def concrete(self) -> rs.Expr:
        """As an owned `Bound<'py, T>` of its concrete type `T` (`typed`)."""
        assert self.typed and self.kind in (Kind.OWNED, Kind.PLACE), self
        if self.kind is Kind.PLACE and not self.temp:
            return rs.MethodCall(self.expr, "clone")
        return self.expr

    def result(self) -> rs.Expr:
        """As a `PyResult<Bound<'py, PyAny>>`, for returning from a function."""
        if self.kind is Kind.OWNED and not self.typed and isinstance(self.expr, rs.Try):
            return self.expr.expr
        if self.kind is Kind.PLACE and not self.caught:
            # Returning moves the local out; nothing runs afterwards.
            expr = rs.MethodCall(self.expr, "into_any") if self.typed else self.expr
            return rs.Call(rs.Path("Ok"), (expr,))
        return rs.Call(rs.Path("Ok"), (self.owned(),))

    def truthy(self) -> rs.Expr:
        """As a Rust `bool` that is Python's truth value of the object."""
        match self.kind:
            case Kind.BOOL:
                return self.expr
            case Kind.NATIVE:
                return rs.Binary("!=", self.expr, rs.IntLit(0))
        return _try(self.method("is_truthy"))


# PyO3 implements `PyCallArgs` and `IntoPyObject` for tuples of up to this many
# elements.
MAX_CALL_ARGS = 12


def _try(expr: rs.Expr) -> rs.Expr:
    return rs.Try(expr)


Operand = ast.expr | Callable[[], Value]
