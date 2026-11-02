"""Lower Python expressions.

Every Python value is a `Bound<'py, PyAny>` in Rust, but a lowered expression
can still take several Rust shapes. `Value` records the shape so that each use
site can pick the most readable spelling: a literal stays a literal where PyO3
accepts any `IntoPyObject`, a parameter is already a `&Bound`, and so on.

Evaluation order follows Python for what M1 supports: a method call evaluates
its receiver before its arguments and arguments left to right, which matches
Python's left-to-right operand evaluation (design §8).
"""

from __future__ import annotations

import ast
from dataclasses import dataclass
from enum import Enum

from steelix.frontend.resolve import Direct, Lookup, resolve_global
from steelix.lower.context import Context
from steelix.rustast import nodes as rs


class Kind(Enum):
    OWNED = "owned"
    """A `Bound<'py, PyAny>` value, usually a call ending in `?`."""
    REF = "ref"
    """A `&Bound<'py, PyAny>`: a parameter or a checked `Option` local."""
    PLACE = "place"
    """A local variable of type `Bound<'py, PyAny>`."""
    LITERAL = "literal"
    """A Rust literal usable as `impl IntoPyObject`; `owned` builds the object."""


@dataclass(frozen=True)
class Value:
    expr: rs.Expr
    kind: Kind
    owned_literal: rs.Expr | None = None

    # --- spellings for the different use sites ---

    def arg(self) -> rs.Expr:
        """As an `impl IntoPyObject` argument of a PyO3 method or call tuple."""
        if self.kind is Kind.PLACE:
            return rs.Ref(self.expr)
        return self.expr

    def ref(self) -> rs.Expr:
        """As a `&Bound<'py, PyAny>` argument of a generated function."""
        match self.kind:
            case Kind.REF:
                return self.expr
            case Kind.LITERAL:
                assert self.owned_literal is not None
                return rs.Ref(self.owned_literal)
        return rs.Ref(self.expr)

    def receiver(self) -> rs.Expr:
        """As the receiver of a PyO3 method call (auto-referenced)."""
        if self.kind is Kind.LITERAL:
            assert self.owned_literal is not None
            return self.owned_literal
        return self.expr

    def owned(self) -> rs.Expr:
        """As an owned `Bound<'py, PyAny>`."""
        match self.kind:
            case Kind.LITERAL:
                assert self.owned_literal is not None
                return self.owned_literal
            case Kind.REF | Kind.PLACE:
                return rs.MethodCall(self.expr, "clone")
        return self.expr

    def result(self) -> rs.Expr:
        """As a `PyResult<Bound<'py, PyAny>>`, for returning from a function."""
        if self.kind is Kind.OWNED and isinstance(self.expr, rs.Try):
            return self.expr.expr
        if self.kind is Kind.PLACE:
            # Returning moves the local out; nothing runs afterwards.
            return rs.Call(rs.Path("Ok"), (self.expr,))
        return rs.Call(rs.Path("Ok"), (self.owned(),))


_BINOPS: dict[type[ast.operator], str] = {
    ast.Add: "add",
    ast.Sub: "sub",
    ast.Mult: "mul",
    ast.MatMult: "matmul",
    ast.Div: "div",
    ast.FloorDiv: "floor_div",
    ast.Mod: "rem",
    ast.Pow: "pow",
    ast.LShift: "lshift",
    ast.RShift: "rshift",
    ast.BitOr: "bitor",
    ast.BitXor: "bitxor",
    ast.BitAnd: "bitand",
}

_COMPARE: dict[type[ast.cmpop], tuple[str, str]] = {
    ast.Lt: ("lt", "Lt"),
    ast.LtE: ("le", "Le"),
    ast.Gt: ("gt", "Gt"),
    ast.GtE: ("ge", "Ge"),
    ast.Eq: ("eq", "Eq"),
    ast.NotEq: ("ne", "Ne"),
}

_I32 = range(-(2**31), 2**31)
_I64 = range(-(2**63), 2**63)
_I128 = range(-(2**127), 2**127)

# PyO3 implements `PyCallArgs` for tuples of up to this many elements.
MAX_CALL_ARGS = 12


def _try(expr: rs.Expr) -> rs.Expr:
    return rs.Try(expr)


def _rt(name: str, *args: rs.Expr) -> rs.Call:
    return rs.Call(rs.Path(f"rt::{name}"), args)


class ExprLowering:
    def __init__(self, ctx: Context) -> None:
        self.ctx = ctx

    def py(self) -> rs.Path:
        return rs.Path(self.ctx.py())

    def globals(self) -> rs.Path:
        return rs.Path(self.ctx.globals())

    # --- entry points ---

    def value(self, expr: ast.expr) -> Value:
        match expr:
            case ast.Constant(value):
                return self.constant(value, expr)
            case ast.UnaryOp(ast.USub(), ast.Constant(value)):
                return self.constant(-value, expr)  # type: ignore[operator]
            case ast.Name(id):
                return self.name(id, expr)
            case ast.BinOp(left, op, right):
                return self.binop(left, op, right)
            case ast.Compare(left, [op], [right]):
                method = _COMPARE[type(op)][1]
                self.ctx.module.uses.add("pyo3::pyclass::CompareOp")
                lhs = self.value(left)
                rhs = self.value(right)
                call = rs.MethodCall(
                    lhs.receiver(), "rich_compare", (rhs.arg(), rs.Path(f"CompareOp::{method}"))
                )
                return Value(_try(call), Kind.OWNED)
            case ast.Call(ast.Name(id), args):
                return self.call(id, args, expr)
        raise self.ctx.unsupported(type(expr).__name__, expr)

    def condition(self, expr: ast.expr) -> rs.Expr:
        """Lower `expr` to a Rust `bool`, as in an `if` or `while` test."""
        match expr:
            case ast.Constant(value):
                return rs.BoolLit(bool(value))
            case ast.Compare(left, [op], [right]):
                lhs = self.value(left)
                rhs = self.value(right)
                return _try(rs.MethodCall(lhs.receiver(), _COMPARE[type(op)][0], (rhs.arg(),)))
        return _try(rs.MethodCall(self.value(expr).receiver(), "is_truthy"))

    # --- constants ---

    def constant(self, value: object, node: ast.expr) -> Value:
        py = self.py
        match value:
            case None:
                return Value(rs.MethodCall(py(), "None"), Kind.LITERAL, _rt("none", py()))
            case bool():
                lit = rs.BoolLit(value)
                return Value(lit, Kind.LITERAL, _rt("bool", py(), lit))
            case int():
                if value in _I32:
                    lit = rs.IntLit(value)
                elif value in _I64:
                    lit = rs.IntLit(value, "i64")
                elif value in _I128:
                    lit = rs.IntLit(value, "i128")
                else:
                    big = _try(_rt("bigint", py(), rs.StrLit(str(value))))
                    return Value(big, Kind.OWNED)
                return Value(lit, Kind.LITERAL, _rt("int", py(), lit))
            case float():
                lit = rs.FloatLit(value)
                return Value(lit, Kind.LITERAL, _rt("float", py(), lit))
            case str():
                if _has_surrogates(value):
                    encoded = value.encode("utf-8", "surrogatepass")
                    call = _rt("str_surrogatepass", py(), rs.ByteStrLit(encoded))
                    return Value(_try(call), Kind.OWNED)
                lit = rs.StrLit(value)
                return Value(lit, Kind.LITERAL, _rt("str", py(), lit))
            case bytes():
                return Value(_rt("bytes", py(), rs.ByteStrLit(value)), Kind.OWNED)
        raise self.ctx.unsupported(f"{type(value).__name__} constant", node)

    # --- names ---

    def name(self, name: str, node: ast.expr) -> Value:
        fn = self.ctx.function
        if fn is not None and fn.scope.is_local(name):
            rust = fn.names[name]
            if name in fn.locals.optional:
                check = _rt("local", rs.Ref(rs.Path(rust)), rs.StrLit(name))
                return Value(_try(check), Kind.REF)
            if name in fn.scope.params and name not in fn.locals.assigned_params:
                return Value(rs.Path(rust), Kind.REF)
            return Value(rs.Path(rust), Kind.PLACE)
        return self.global_name(name)

    def global_name(self, name: str) -> Value:
        resolution = resolve_global(self.ctx.module.bindings, name)
        if resolution is Lookup.BUILTIN:
            return Value(_try(_rt("builtin", self.py(), rs.StrLit(name))), Kind.OWNED)
        return Value(_try(_rt("global", self.globals(), rs.StrLit(name))), Kind.OWNED)

    # --- operators ---

    def binop(self, left: ast.expr, op: ast.operator, right: ast.expr) -> Value:
        method = _BINOPS[type(op)]
        lhs = self.value(left)
        rhs = self.value(right)
        args: tuple[rs.Expr, ...] = (rhs.arg(),)
        if method == "pow":
            args += (rs.MethodCall(self.py(), "None"),)
        return Value(_try(rs.MethodCall(lhs.receiver(), method, args)), Kind.OWNED)

    # --- calls ---

    def call(self, name: str, args: list[ast.expr], node: ast.Call) -> Value:
        if len(args) > MAX_CALL_ARGS:
            raise self.ctx.unsupported(f"call with more than {MAX_CALL_ARGS} arguments", node)
        fn = self.ctx.function
        is_global = fn is None or not fn.scope.is_local(name)
        bindings = self.ctx.module.bindings
        if is_global and not bindings.is_module_bound(name) and not args:
            if name == "globals":
                owned = rs.MethodCall(rs.MethodCall(self.globals(), "clone"), "into_any")
                return Value(owned, Kind.OWNED)
            if name in ("locals", "vars"):
                # They read the calling frame, which transpiled code does not have.
                raise self.ctx.unsupported(f"{name}()", node)
        if is_global:
            resolution = resolve_global(
                self.ctx.module.bindings,
                name,
                call_arity=len(args),
                module_index=self.ctx.statement_index,
            )
            if isinstance(resolution, Direct):
                return self.direct_call(resolution.fn, args)
            if resolution is Lookup.BUILTIN:
                if name == "print":
                    values = [self.value(a).arg() for a in args]
                    call = rs.Call(
                        rs.Path("rt::builtins::print"), (self.py(), rs.Tuple(tuple(values)))
                    )
                    return Value(_try(call), Kind.OWNED)
        callee = self.value(node.func)
        values = tuple(self.value(a).arg() for a in args)
        if values:
            call = rs.MethodCall(callee.receiver(), "call1", (rs.Tuple(values),))
        else:
            call = rs.MethodCall(callee.receiver(), "call0")
        return Value(_try(call), Kind.OWNED)

    def direct_call(self, target: ast.FunctionDef, args: list[ast.expr]) -> Value:
        module = self.ctx.module
        call_args: list[rs.Expr] = [self.py()]
        if target in module.callgraph.needs_globals:
            call_args.append(self.globals())
        call_args += [self.value(a).ref() for a in args]
        call = rs.Call(rs.Path(module.fn_names[target.name]), tuple(call_args))
        return Value(_try(call), Kind.OWNED)


def _has_surrogates(value: str) -> bool:
    return any(0xD800 <= ord(ch) <= 0xDFFF for ch in value)
