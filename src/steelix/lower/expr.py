"""Lower Python expressions.

Every Python value is a `Bound<'py, PyAny>` in Rust, but a lowered expression
can still take several Rust shapes. `Value` records the shape so that each use
site can pick the most readable spelling: a literal stays a literal where PyO3
accepts any `IntoPyObject`, a parameter is already a `&Bound`, and so on.

Evaluation order (design §8). Rust evaluates a method call's receiver, then
its arguments left to right, which is Python's order for most expressions.
Where the Rust shape evaluates operands in a different order (`x in y` is
`y.contains(x)`, `a[i] = v` is `a.set_item(i, v)`), operands that could
observe each other are first evaluated into temporaries, in Python order.
Locals and literals can not observe anything, so they never need one.

Temporaries are `let` statements collected in `ExprLowering.prelude`, which
the statement lowering emits before the statement. When an operand needs such
statements, every operand evaluated before it in Python order must move into
the prelude as well (`operands` takes care of this). Code that only runs
conditionally, such as the right side of `and`, gets its own prelude, which
ends up in a block of the generated `if`.
"""

from __future__ import annotations

import ast
from collections.abc import Callable, Iterator
from contextlib import contextmanager
from dataclasses import dataclass, replace
from enum import Enum

from steelix.frontend.bindings import is_immutable_constant
from steelix.frontend.resolve import Direct, Lookup, resolve_call, resolve_global
from steelix.frontend.scope import FunctionNode, default_of, params_of
from steelix.lower.comprehension import ComprehensionLowering, ComprehensionVariable
from steelix.lower.context import NAMESPACE, Context
from steelix.lower.names import rust_ident
from steelix.rustast import nodes as rs
from steelix.rustast.printer import string_literal
from steelix.rustast.pyo3 import INFALLIBLE_LEN, must_erase


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
        """As an `impl IntoPyObject` argument of a PyO3 method or call tuple."""
        if self.kind is Kind.PLACE:
            return rs.Ref(self.expr)
        return self.expr

    def ref(self) -> rs.Expr:
        """As a `&Bound<'py, PyAny>`, e.g. an argument of a generated function."""
        match self.kind:
            case Kind.REF:
                # `&Bound<PyTuple>` only coerces to `&Bound<PyAny>` in some places.
                return rs.MethodCall(self.expr, "as_any") if self.typed else self.expr
            case Kind.LITERAL | Kind.BOOL | Kind.NATIVE:
                assert self.owned_literal is not None
                return rs.Ref(self.owned_literal)
        if self.typed:
            # `&PyList::new(..)?` does not coerce: `?` needs the exact type.
            return rs.MethodCall(self.expr, "as_any")
        return rs.Ref(self.expr)

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

    def result(self) -> rs.Expr:
        """As a `PyResult<Bound<'py, PyAny>>`, for returning from a function."""
        if self.kind is Kind.OWNED and not self.typed and isinstance(self.expr, rs.Try):
            return self.expr.expr
        if self.kind is Kind.PLACE:
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

# Runtime helpers for augmented assignment, named after the `operator` module.
IN_PLACE: dict[type[ast.operator], str] = {
    ast.Add: "iadd",
    ast.Sub: "isub",
    ast.Mult: "imul",
    ast.MatMult: "imatmul",
    ast.Div: "itruediv",
    ast.FloorDiv: "ifloordiv",
    ast.Mod: "imod",
    ast.Pow: "ipow",
    ast.LShift: "ilshift",
    ast.RShift: "irshift",
    ast.BitOr: "ior",
    ast.BitXor: "ixor",
    ast.BitAnd: "iand",
}

_UNARY: dict[type[ast.unaryop], str] = {ast.USub: "neg", ast.UAdd: "pos", ast.Invert: "bitnot"}

_COMPARE: dict[type[ast.cmpop], tuple[str, str, str]] = {
    ast.Lt: ("lt", "Lt", "<"),
    ast.LtE: ("le", "Le", "<="),
    ast.Gt: ("gt", "Gt", ">"),
    ast.GtE: ("ge", "Ge", ">="),
    ast.Eq: ("eq", "Eq", "=="),
    ast.NotEq: ("ne", "Ne", "!="),
}

_I32 = range(-(2**31), 2**31)
_I64 = range(-(2**63), 2**63)
_I128 = range(-(2**127), 2**127)

# PyO3 implements `PyCallArgs` and `IntoPyObject` for tuples of up to this many
# elements.
MAX_CALL_ARGS = 12

# Builtins with a direct PyO3 spelling, by name and number of arguments. Only
# used when the name can only be the builtin (design §12).
_SPECIAL_BUILTINS = {
    ("len", 1),
    ("isinstance", 2),
    ("str", 1),
    ("repr", 1),
    ("bool", 1),
    ("abs", 1),
    ("type", 1),
    ("iter", 1),
}


def _try(expr: rs.Expr) -> rs.Expr:
    return rs.Try(expr)


def _rt(name: str, *args: rs.Expr) -> rs.Call:
    return rs.Call(rs.Path(f"rt::{name}"), args)


def negate(cond: rs.Expr) -> rs.Expr:
    if isinstance(cond, rs.Unary) and cond.op == "!":
        return cond.expr
    if isinstance(cond, rs.BoolLit):
        return rs.BoolLit(not cond.value)
    return rs.Unary("!", cond)


Operand = ast.expr | Callable[[], Value]


class ExprLowering(ComprehensionLowering):
    def __init__(self, ctx: Context) -> None:
        self.ctx = ctx
        self.prelude: list[rs.Stmt] = []
        """Statements that must run before the expression being lowered."""
        self.line: int | None = None
        """The traceback line of the frame where the expression is evaluated,
        if known (`ComprehensionLowering.at_line`)."""
        self.markers = 0
        """Number of `frame.line(n)` emitted so far."""
        self.comprehensions: list[dict[str, ComprehensionVariable]] = []
        """The variables of the comprehensions being lowered, innermost last."""

    def py(self) -> rs.Path:
        return rs.Path(self.ctx.py())

    def module(self) -> rs.Path:
        return rs.Path(self.ctx.context())

    def module_call(self, method: str, *args: rs.Expr) -> rs.MethodCall:
        """`m.method(args)` on the `rt::Module` context."""
        return rs.MethodCall(self.module(), method, args)

    def scope_call(self, method: str, *args: rs.Expr) -> rs.MethodCall:
        """`set`, `del` or `unbind` a name of the current non-function scope:
        `ns.set(..)` in a class body, `m.set_global(..)` in module code."""
        if self.ctx.class_body is not None:
            return rs.MethodCall(rs.Path(NAMESPACE), method, args)
        return self.module_call(f"{method}_global", *args)

    def use(self, path: str) -> None:
        self.ctx.module.uses.add(path)

    @contextmanager
    def fresh(self) -> Iterator[list[rs.Stmt]]:
        """Collect the prelude of code that runs separately, e.g. conditionally."""
        saved = self.prelude
        self.prelude = []
        try:
            yield self.prelude
        finally:
            self.prelude = saved

    # --- temporaries and evaluation order ---

    def hoist(self, value: Value, role: str = "value", at: int | None = None) -> Value:
        """Evaluate `value` into a temporary now, unless that makes no difference.

        The temporary is named after the expression if possible, else `role`.
        """
        if value.is_pure:
            return value
        name = self.ctx.temp(value.base or role)
        init = value.expr
        if value.kind is Kind.PLACE:
            # A local that `:=` may assign before the value is used.
            init = rs.MethodCall(init, "clone")
        stmt = rs.Let(name, init=init)
        if at is None:
            self.prelude.append(stmt)
        else:
            self.prelude.insert(at, stmt)
        path = rs.Path(name)
        match value.kind:
            case Kind.BOOL:
                return Value(path, Kind.BOOL, _rt("bool", self.py(), path), pure=True)
            case Kind.NATIVE:
                return Value(path, Kind.NATIVE, _rt("int", self.py(), path), pure=True)
            case Kind.REF:
                return Value(path, Kind.REF)
        return Value(path, Kind.PLACE, typed=value.typed, base=value.base, temp=True)

    def operands(self, items: list[Operand], first: list[Value] | None = None) -> list[Value]:
        """Lower operands in Python order. `first` are operands lowered already.

        If an operand needs prelude statements, the operands before it are
        evaluated into temporaries ahead of those statements.
        """
        values = list(first or [])
        for item in items:
            start = len(self.prelude)
            value = item() if callable(item) else self.value(item)
            if len(self.prelude) > start:
                at = start
                for i, earlier in enumerate(values):
                    if not earlier.is_pure:
                        values[i] = self.hoist(earlier, "value", at)
                        at += 1
            values.append(value)
        return values

    def in_rust_order(self, values: list[Value], order: list[int], roles: list[str]) -> list[Value]:
        """Make `values` (in Python order) safe to evaluate in Rust `order`.

        A value that Rust evaluates after a later Python operand that is not
        pure is moved into a temporary, together with every operand before it.
        """
        position = {index: pos for pos, index in enumerate(order)}
        last = -1
        for i, value in enumerate(values):
            if value.is_pure:
                continue
            for j in range(i + 1, len(values)):
                if not values[j].is_pure and position[j] < position[i]:
                    last = i
        return [
            self.hoist(v, role) if i <= last else v
            for i, (v, role) in enumerate(zip(values, roles, strict=True))
        ]

    def hoist_all(self, values: list[Value], roles: list[str]) -> list[Value]:
        """Evaluate every operand into a temporary, e.g. because it is used twice."""
        return [self.hoist(v, role) for v, role in zip(values, roles, strict=True)]

    # --- entry points ---

    def value(self, expr: ast.expr) -> Value:
        value = self._value(expr)
        if isinstance(expr, ast.Name | ast.Attribute):
            # A readable name in case the value becomes a temporary.
            return replace(value, base=_expr_base(expr))
        return value

    def _value(self, expr: ast.expr) -> Value:
        match expr:
            case ast.Constant(value):
                return self.constant(value, expr)
            case ast.UnaryOp(ast.USub(), ast.Constant(value)) if _is_number(value):
                return self.constant(-value, expr)  # type: ignore[operator]
            case ast.UnaryOp(ast.Not(), operand):
                return self.bool_value(negate(self.condition(operand)))
            case ast.UnaryOp(op, operand):
                inner = self.value(operand)
                call = inner.method(_UNARY[type(op)])
                return Value(_try(call), Kind.OWNED)
            case ast.Name(id):
                return self.name(id, expr)
            case ast.BinOp(left, op, right):
                return self.binop(left, op, right)
            case ast.BoolOp(op, values):
                return self.boolop(isinstance(op, ast.And), values)
            case ast.Compare(left, ops, comparators):
                return self.compare(left, ops, comparators)
            case ast.IfExp(test, body, orelse):
                cond = self.condition(test)
                then = self.branch(body)
                other = self.branch(orelse)
                return Value(_if_expr(cond, then, other), Kind.OWNED)
            case ast.Call(func, args, keywords):
                return self.call(func, args, keywords, expr)
            case ast.Lambda():
                from steelix.lower.function import lower_nested

                return self.function_value(expr, lower_nested(self.ctx, expr))
            case ast.Attribute(obj, attr):
                inner = self.value(obj)
                call = inner.method("getattr", rs.StrLit(attr))
                return Value(_try(call), Kind.OWNED)
            case ast.Subscript(obj, index):
                container, key = self.operands([obj, index])
                call = container.method("get_item", key.arg())
                return Value(_try(call), Kind.OWNED)
            case ast.Slice(lower, upper, step):
                return self.slice(lower, upper, step)
            case ast.List(elts):
                return self.sequence("PyList", elts)
            case ast.Tuple(elts):
                return self.sequence("PyTuple", elts)
            case ast.Set(elts):
                return self.sequence("PySet", elts)
            case ast.Dict(keys, values):
                return self.dict(keys, values)  # type: ignore[arg-type]
            case ast.JoinedStr(values):
                return self.fstring(values)
            case ast.ListComp() | ast.SetComp() | ast.DictComp():
                return self.comprehension(expr)
            case ast.GeneratorExp():
                return self.generator_expression(expr)
            case ast.NamedExpr():
                return self.named_expr(expr)
        raise self.ctx.unsupported(type(expr).__name__, expr)

    def bool_value(self, expr: rs.Expr, pure: bool | None = None) -> Value:
        return Value(expr, Kind.BOOL, _rt("bool", self.py(), expr), pure=pure)

    def condition(self, expr: ast.expr) -> rs.Expr:
        """Lower `expr` to a Rust `bool`, as in an `if` or `while` test."""
        match expr:
            case ast.Constant(value):
                return rs.BoolLit(bool(value))
            case ast.UnaryOp(ast.Not(), operand):
                return negate(self.condition(operand))
            case ast.BoolOp(op, values):
                is_and = isinstance(op, ast.And)
                result = self.condition(values[0])
                for value in values[1:]:
                    with self.fresh() as pre:
                        rhs = self.condition(value)
                    if pre:
                        rhs = rs.BlockExpr(rs.Block(tuple(pre), rhs))
                    result = rs.Binary("&&" if is_and else "||", result, rhs)
                return result
            case ast.Compare(left, ops, comparators):
                return self.compare_condition(left, ops, comparators)
            case ast.IfExp(test, body, orelse):
                cond = self.condition(test)
                with self.fresh() as pre_then:
                    then = self.condition(body)
                with self.fresh() as pre_else:
                    other = self.condition(orelse)
                return rs.IfExpr(
                    cond, rs.Block(tuple(pre_then), then), rs.Block(tuple(pre_else), other)
                )
        return self.value(expr).truthy()

    def branch(self, expr: ast.expr) -> rs.Block:
        """A conditionally evaluated expression as a block yielding an owned value."""
        with self.fresh() as pre:
            value = self.value(expr)
        return rs.Block(tuple(pre), value.owned())

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
                return Value(_rt("bytes", py(), rs.ByteStrLit(value)), Kind.OWNED, pure=True)
        if value is Ellipsis:
            ellipsis = rs.MethodCall(py(), "Ellipsis")
            owned = rs.MethodCall(ellipsis, "into_bound", (py(),))
            return Value(ellipsis, Kind.LITERAL, owned)
        raise self.ctx.unsupported(f"{type(value).__name__} constant", node)

    # --- names ---

    def name(self, name: str, node: ast.expr) -> Value:
        var = self.comprehension_variable(name)
        if var is not None:
            if var.cell:
                return Value(_try(rs.MethodCall(rs.Path(var.rust), "get")), Kind.OWNED)
            return Value(rs.Path(var.rust), Kind.PLACE)
        fn = self.ctx.function
        if fn is None and self.ctx.class_body is not None:
            if self.comprehensions:
                # A comprehension in a class body does not see the class
                # namespace (only its first iterable does, evaluated outside).
                return self.global_name(name)
            # `LOAD_NAME`: the class namespace, then globals and builtins.
            get = rs.MethodCall(rs.Path(NAMESPACE), "get", (rs.StrLit(name),))
            return Value(_try(get), Kind.OWNED)
        if fn is not None and not fn.scope.is_global(name):
            rust = fn.names[name]
            if fn.is_cell(name):
                return Value(_try(rs.MethodCall(rs.Path(rust), "get")), Kind.OWNED)
            if fn.scope.is_free(name):
                return Value(rs.Path(rust), Kind.REF)
            if name in fn.locals.optional:
                check = _rt("local", rs.Ref(rs.Path(rust)), rs.StrLit(name))
                return Value(_try(check), Kind.REF)
            if name in fn.locals.with_params:
                return Value(rs.Path(rust), Kind.REF)  # the parameter of a `with` closure
            if name in fn.scope.params and name not in fn.locals.assigned_params:
                typed = {fn.varargs: "PyTuple", fn.varkw: "PyDict"}.get(name)
                return Value(rs.Path(rust), Kind.REF, typed=typed)
            # A later `:=` in the same expression may assign it.
            pure = False if name in fn.walrus_targets else None
            return Value(rs.Path(rust), Kind.PLACE, pure=pure)
        return self.global_name(name)

    def global_name(self, name: str) -> Value:
        resolution = resolve_global(self.ctx.module.bindings, name)
        if resolution is Lookup.BUILTIN:
            return Value(_try(_rt("builtin", self.py(), rs.StrLit(name))), Kind.OWNED)
        return Value(_try(self.module_call("global", rs.StrLit(name))), Kind.OWNED)

    # --- operators ---

    def binop(self, left: ast.expr, op: ast.operator, right: ast.expr) -> Value:
        method = _BINOPS[type(op)]
        lhs, rhs = self.operands([left, right])
        args: tuple[rs.Expr, ...] = (rhs.arg(),)
        if method == "pow":
            args += (rs.MethodCall(self.py(), "None"),)
        return Value(_try(lhs.method(method, *args)), Kind.OWNED)

    def boolop(self, is_and: bool, values: list[ast.expr]) -> Value:
        """`a and b` / `a or b`: the first operand that decides, as an object."""
        if len(values) == 1:
            return self.value(values[0])
        if isinstance(values[0], ast.Constant):
            # A constant decides at transpile time.
            if bool(values[0].value) != is_and:
                return self.value(values[0])
            return self.boolop(is_and, values[1:])
        first = self.value(values[0])
        if first.kind is Kind.BOOL:
            # A Rust bool decides by itself; short-circuiting yields its value.
            cond = first.expr
            decided = _rt("bool", self.py(), rs.BoolLit(not is_and))
        else:
            first = self.hoist(first)
            cond = first.truthy()
            decided = first.owned()
        with self.fresh() as pre:
            rest = self.boolop(is_and, values[1:])
        rest_block = rs.Block(tuple(pre), rest.owned())
        decided_block = rs.Block((), decided)
        if is_and:
            return Value(_if_expr(cond, rest_block, decided_block), Kind.OWNED)
        return Value(_if_expr(cond, decided_block, rest_block), Kind.OWNED)

    # --- comparisons ---

    def compare(self, left: ast.expr, ops: list[ast.cmpop], comparators: list[ast.expr]) -> Value:
        lhs, rhs = self.operands([left, comparators[0]])
        if len(ops) == 1:
            return self.compare_values(ops[0], lhs, rhs)
        # `a < b < c`: `b` is evaluated once and used twice.
        if not rhs.is_pure:
            lhs, rhs = self.hoist_all([lhs, rhs], ["lhs", "rhs"])
        first = self.compare_values(ops[0], lhs, rhs)

        def rest() -> Value:
            return self.compare_rest(rhs, ops[1:], comparators[1:])

        return self.and_then(first, rest)

    def compare_rest(self, lhs: Value, ops: list[ast.cmpop], comparators: list[ast.expr]) -> Value:
        rhs = self.value(comparators[0])
        if len(ops) == 1:
            return self.compare_values(ops[0], lhs, rhs)
        rhs = self.hoist(rhs, "rhs")
        first = self.compare_values(ops[0], lhs, rhs)
        return self.and_then(first, lambda: self.compare_rest(rhs, ops[1:], comparators[1:]))

    def and_then(self, first: Value, rest: Callable[[], Value]) -> Value:
        """`first and rest()` with value semantics, `rest` lowered conditionally."""
        if first.kind is Kind.BOOL:
            cond = first.expr
            decided = _rt("bool", self.py(), rs.BoolLit(False))
        else:
            first = self.hoist(first, "result")
            cond = first.truthy()
            decided = first.owned()
        with self.fresh() as pre:
            value = rest()
        return Value(
            _if_expr(cond, rs.Block(tuple(pre), value.owned()), rs.Block((), decided)),
            Kind.OWNED,
        )

    def compare_values(self, op: ast.cmpop, lhs: Value, rhs: Value) -> Value:
        """One comparison as a Python object (or a Rust `bool` where it always is one)."""
        match op:
            case ast.In() | ast.NotIn():
                lhs, rhs = self.in_rust_order([lhs, rhs], [1, 0], ["item", "container"])
                contains = _try(rhs.method("contains", lhs.arg()))
                return self.bool_value(contains if isinstance(op, ast.In) else negate(contains))
            case ast.Is() | ast.IsNot():
                cond = self._identity(lhs, rhs)
                pure = lhs.is_pure and rhs.is_pure
                return self.bool_value(cond if isinstance(op, ast.Is) else negate(cond), pure)
        self.use("pyo3::pyclass::CompareOp")
        method = _COMPARE[type(op)][1]
        call = lhs.method("rich_compare", rhs.arg(), rs.Path(f"CompareOp::{method}"))
        return Value(_try(call), Kind.OWNED)

    def _identity(self, lhs: Value, rhs: Value) -> rs.Expr:
        if _is_none_literal(rhs):
            return lhs.method("is_none")
        if _is_none_literal(lhs):
            return rhs.method("is_none")
        return lhs.method("is", rhs.identity())

    def compare_condition(
        self, left: ast.expr, ops: list[ast.cmpop], comparators: list[ast.expr]
    ) -> rs.Expr:
        lhs, rhs = self.operands([left, comparators[0]])
        if len(ops) == 1:
            return self.compare_bool(ops[0], lhs, rhs)
        # `a < b < c`: `b` is evaluated once and used twice.
        if not rhs.is_pure:
            lhs, rhs = self.hoist_all([lhs, rhs], ["lhs", "rhs"])
        conds = [self.compare_bool(ops[0], lhs, rhs)]
        conds += self.compare_condition_rest(rhs, ops[1:], comparators[1:])
        return _all(conds)

    def compare_condition_rest(
        self, lhs: Value, ops: list[ast.cmpop], comparators: list[ast.expr]
    ) -> list[rs.Expr]:
        """The comparisons of a chain after the first, as conjuncts that only
        run if the earlier ones are true. A temporary for a middle operand is
        declared in a block that also holds every later comparison using it."""
        with self.fresh() as pre:
            rhs = self.value(comparators[0])
            if len(ops) > 1:
                rhs = self.hoist(rhs, "rhs")
            conds = [self.compare_bool(ops[0], lhs, rhs)]
            if len(ops) > 1:
                conds += self.compare_condition_rest(rhs, ops[1:], comparators[1:])
        if pre:
            return [rs.BlockExpr(rs.Block(tuple(pre), _all(conds)))]
        return conds

    def compare_bool(self, op: ast.cmpop, lhs: Value, rhs: Value) -> rs.Expr:
        """One comparison as a Rust `bool`, as in a condition."""
        if isinstance(op, ast.In | ast.NotIn | ast.Is | ast.IsNot):
            return self.compare_values(op, lhs, rhs).expr
        method, _, symbol = _COMPARE[type(op)]
        if _native_int_comparable(lhs, rhs):
            # `len(xs) > 3`: both sides are plain Rust integers.
            return rs.Binary(symbol, lhs.expr, rhs.expr)
        return _try(lhs.method(method, rhs.arg()))

    # --- subscripts and displays ---

    def slice(self, lower: ast.expr | None, upper: ast.expr | None, step: ast.expr | None) -> Value:
        none = self.constant(None, ast.Constant(None))
        parts = [p if p is not None else (lambda: none) for p in (lower, upper, step)]
        if step is None:
            parts = parts[:2]
        values = self.operands(parts)  # type: ignore[arg-type]
        helper = "slice" if step is None else "slice_step"
        call = _rt(helper, self.py(), *(v.arg() for v in values))
        # Building a slice of plain values can not fail or have effects.
        pure = all(v.is_pure for v in values)
        return Value(_try(call), Kind.OWNED, typed="PySlice", pure=pure)

    def elements(self, values: list[Value]) -> list[rs.Expr]:
        """Elements of a Rust array, which must all have the same type."""
        if values and all(v.kind is Kind.LITERAL for v in values):
            kinds = {_literal_type(v.expr) for v in values}
            if len(kinds) == 1 and None not in kinds:
                return [v.expr for v in values]
        return [v.ref() for v in values]

    def sequence(self, type_name: str, elts: list[ast.expr]) -> Value:
        """A list, tuple or set display."""
        self.use(f"pyo3::types::{type_name}")
        if not elts and type_name != "PySet":
            call: rs.Expr = rs.Call(rs.Path(f"{type_name}::empty"), (self.py(),))
            return Value(call, Kind.OWNED, typed=type_name)
        values = self.operands(list(elts))
        array = rs.Array(tuple(self.elements(values)))
        call = _try(rs.Call(rs.Path(f"{type_name}::new"), (self.py(), array)))
        return Value(call, Kind.OWNED, typed=type_name)

    def dict(self, keys: list[ast.expr], values: list[ast.expr]) -> Value:
        if not keys:
            self.use("pyo3::types::PyDict")
            return Value(rs.Call(rs.Path("PyDict::new"), (self.py(),)), Kind.OWNED, typed="PyDict")
        self.use("pyo3::types::IntoPyDict")
        items = self.operands([e for pair in zip(keys, values, strict=True) for e in pair])
        lowered_keys = self.elements(items[0::2])
        lowered_values = self.elements(items[1::2])
        pairs = rs.Array(
            tuple(rs.Tuple((k, v)) for k, v in zip(lowered_keys, lowered_values, strict=True))
        )
        call = _try(rs.MethodCall(pairs, "into_py_dict", (self.py(),)))
        return Value(call, Kind.OWNED, typed="PyDict")

    # --- f-strings ---

    def fstring(self, parts: list[ast.expr]) -> Value:
        pieces: list[Operand] = []
        for part in parts:
            match part:
                case ast.Constant(str() as text):
                    pieces.append(lambda text=text, part=part: self.constant(text, part))
                case ast.FormattedValue():
                    pieces.append(lambda part=part: self.formatted(part))
        if not pieces:
            return self.constant("", ast.Constant(""))
        values = self.operands(pieces)
        if len(values) == 1:
            return values[0]
        return self.join(values)

    def join(self, values: list[Value]) -> Value:
        """Concatenate f-string pieces, nesting when a tuple would be too long."""
        if len(values) > MAX_CALL_ARGS:
            head = self.join(values[: MAX_CALL_ARGS - 1])
            values = [head, *values[MAX_CALL_ARGS - 1 :]]
            return self.join(values)
        args = rs.Tuple(tuple(v.arg() for v in values))
        return Value(_try(_rt("fstring", self.py(), args)), Kind.OWNED)

    def formatted(self, node: ast.FormattedValue) -> Value:
        value = self.value(node.value)
        match chr(node.conversion) if node.conversion >= 0 else None:
            case "s":
                value = Value(_try(value.method("str")), Kind.OWNED, typed="PyString")
            case "r":
                value = Value(_try(value.method("repr")), Kind.OWNED, typed="PyString")
            case "a":
                value = Value(_try(_rt("ascii", value.ref())), Kind.OWNED, typed="PyString")
        spec = node.format_spec
        if spec is None:
            if node.conversion >= 0:
                return value
            spec_value = self.constant("", ast.Constant(""))
        elif all(isinstance(p, ast.Constant) for p in spec.values):
            text = "".join(p.value for p in spec.values if isinstance(p, ast.Constant))
            spec_value = self.constant(text, spec)
        else:
            [value, spec_value] = self.operands([spec], [value])
        call = _rt("format", value.ref(), spec_value.arg())
        return Value(_try(call), Kind.OWNED)

    # --- calls ---

    def call(
        self, func: ast.expr, args: list[ast.expr], keywords: list[ast.keyword], node: ast.Call
    ) -> Value:
        plain = not keywords and not any(isinstance(a, ast.Starred) for a in args)
        if (
            self.ctx.class_body is not None
            and self.ctx.function is None
            and isinstance(func, ast.Name)
            and func.id in _FRAME_BUILTINS
            and len(args) + len(keywords) < (2 if func.id in ("exec", "eval") else 1)
        ):
            # They look at the calling frame, which a native class body does not have.
            raise self.ctx.unsupported(f"{func.id}() in a class body", node)
        if isinstance(func, ast.Name) and self.is_global(func.id):
            special = self.global_call(func.id, node, plain)
            if special is not None:
                return special
        if not plain:
            return self.call_macro(func, args, keywords)
        if len(args) > MAX_CALL_ARGS:
            raise self.ctx.unsupported(f"call with more than {MAX_CALL_ARGS} arguments", node)
        if isinstance(func, ast.Attribute):
            return self.method_call(func, args)
        callee, *values = self.operands([func, *args])
        return self.call_value(callee, values)

    def call_value(self, callee: Value, values: list[Value]) -> Value:
        if values:
            tuple_ = rs.Tuple(tuple(v.arg() for v in values))
            call = callee.method("call1", tuple_)
        else:
            call = callee.method("call0")
        return Value(_try(call), Kind.OWNED)

    def call_macro(
        self, func: ast.expr, args: list[ast.expr], keywords: list[ast.keyword]
    ) -> Value:
        """A call with keyword arguments or unpacking: `rt::call!(f, x, *xs, k = v)`.

        Python evaluates the callee, the positional arguments, then the
        keyword arguments, which is the order the macro evaluates them in.
        """
        if isinstance(func, ast.Attribute):
            # `o.m(..)` looks the method up first.
            def method() -> Value:
                obj = self.value(func.value)
                getattr_ = obj.method("getattr", rs.StrLit(func.attr))
                return Value(_try(getattr_), Kind.OWNED, base=func.attr)

            first: Operand = method
        else:
            first = func
        items: list[Operand] = [first]
        items += [a.value if isinstance(a, ast.Starred) else a for a in args]
        items += [k.value for k in keywords]
        callee, *values = self.operands(items)
        parts: list[rs.Expr] = []
        for arg, value in zip(args, values, strict=False):
            if isinstance(arg, ast.Starred):
                parts.append(rs.Tokens(("*", value.bound())))
            else:
                parts.append(value.arg())
        for keyword, value in zip(keywords, values[len(args) :], strict=True):
            if keyword.arg is None:
                parts.append(rs.Tokens(("**", value.bound())))
            else:
                parts.append(rs.Tokens((f"{_keyword(keyword.arg)} = ", value.arg())))
        macro = rs.MacroCall("rt::call", (callee.bound(), *parts))
        return Value(_try(macro), Kind.OWNED)

    def method_call(self, func: ast.Attribute, args: list[ast.expr]) -> Value:
        """`o.m(args)`: `call_method1` when that keeps Python's order, which
        looks the method up before evaluating the arguments."""
        obj = self.value(func.value)
        start = len(self.prelude)
        getattr_ = obj.method("getattr", rs.StrLit(func.attr))
        method = Value(_try(getattr_), Kind.OWNED, base=func.attr)
        callee, *values = self.operands(list(args), [method])
        if len(self.prelude) == start and all(v.is_pure for v in values):
            name = rs.StrLit(func.attr)
            if values:
                tuple_ = rs.Tuple(tuple(v.arg() for v in values))
                call = obj.method("call_method1", name, tuple_)
            else:
                call = obj.method("call_method0", name)
            return Value(_try(call), Kind.OWNED)
        return self.call_value(callee, values)

    def is_global(self, name: str) -> bool:
        """Whether `name` is looked up in the module globals (and builtins)."""
        if self.comprehension_variable(name) is not None:
            return False
        fn = self.ctx.function
        if fn is None:
            return self.ctx.class_body is None or bool(self.comprehensions)
        return fn.scope.is_global(name)

    def global_call(self, name: str, node: ast.Call, plain: bool) -> Value | None:
        """Calls of module functions and builtins with a direct Rust spelling."""
        bindings = self.ctx.module.bindings
        args = node.args
        if name == "super" and not args and not node.keywords:
            if not bindings.is_module_bound(name):
                return self.zero_arg_super(node)
        if self.comprehensions and _reads_frame(name, node, bindings.is_module_bound(name)):
            # In a comprehension the calling frame has the comprehension's
            # variables as locals too.
            raise self.ctx.unsupported(f"{name}() in a comprehension", node)
        if not bindings.is_module_bound(name) and not args and not node.keywords:
            # At module level the local namespace is the globals dict.
            namespace_name = name == "globals" or (
                name in ("locals", "vars") and self.ctx.at_module_level
            )
            if namespace_name:
                namespace = self.module_call("globals")
                owned = rs.MethodCall(rs.MethodCall(namespace, "clone"), "into_any")
                return Value(owned, Kind.OWNED)
            if name == "dir" and self.ctx.at_module_level:
                return Value(_try(self.module_call("dir")), Kind.OWNED)
            if name in ("locals", "vars", "dir"):
                # In a function they read the calling frame, which transpiled
                # code does not have.
                raise self.ctx.unsupported(f"{name}() in a function", node)
        if (
            name in ("exec", "eval")
            and not bindings.is_module_bound(name)
            and len(args) == 1
            and not isinstance(args[0], ast.Starred)
            and not node.keywords
        ):
            # Without a namespace they run in the calling frame, which
            # transpiled code does not have. At module level that is the
            # globals dict, which can be passed explicitly.
            if not self.ctx.at_module_level or bindings.is_module_bound("globals"):
                raise self.ctx.unsupported(f"{name}() without a namespace in a function", node)
            namespace = ast.copy_location(ast.Call(ast.Name("globals", ast.Load()), [], []), node)
            explicit = ast.copy_location(ast.Call(node.func, [args[0], namespace], []), node)
            return self.call(node.func, explicit.args, [], explicit)
        resolution = resolve_call(bindings, name, node, module_index=self.ctx.statement_index)
        if isinstance(resolution, Direct):
            return self.direct_call(resolution, node)
        if resolution is not Lookup.BUILTIN or not plain:
            return None
        if name == "print":
            values = self.operands(list(args))
            tuple_ = rs.Tuple(tuple(v.arg() for v in values))
            call = rs.Call(rs.Path("rt::builtins::print"), (self.py(), tuple_))
            return Value(_try(call), Kind.OWNED)
        if (name, len(args)) in _SPECIAL_BUILTINS:
            return self.builtin_call(name, self.operands(list(args)))
        return None

    def zero_arg_super(self, node: ast.Call) -> Value:
        """`super()` in a method: `rt::super_(&__class__, slf)`, the class
        from the `__class__` cell and the first argument of the function."""
        fn = self.ctx.function
        if fn is None or not fn.scope.is_free("__class__"):
            raise self.ctx.unsupported("super() without arguments outside a method", node)
        positional = fn.node.args.posonlyargs + fn.node.args.args
        if not positional:
            raise self.ctx.unsupported("super() without arguments and no first argument", node)
        first = self.name(positional[0].arg, node)
        cell = rs.Path(fn.names["__class__"])
        return Value(_try(_rt("super_", cell, first.ref())), Kind.OWNED)

    def builtin_call(self, name: str, args: list[Value]) -> Value:
        """Builtins whose PyO3 method does exactly what the builtin does."""
        arg = args[0]
        match name:
            case "len":
                length: rs.Expr = arg.method("len")
                if arg.typed not in INFALLIBLE_LEN:
                    length = _try(length)
                return Value(length, Kind.NATIVE, _rt("int", self.py(), length), base="len")
            case "isinstance":
                check = _try(arg.method("is_instance", args[1].ref()))
                return self.bool_value(check)
            case "bool":
                return self.bool_value(arg.truthy())
            case "type":
                return Value(arg.method("get_type"), Kind.OWNED, typed="PyType")
            case "iter":
                call = _try(arg.method("try_iter"))
                return Value(call, Kind.OWNED, typed="PyIterator")
            case "str" | "repr":
                call = _try(arg.method(name))
                return Value(call, Kind.OWNED, typed="PyString")
            case "abs":
                return Value(_try(arg.method("abs")), Kind.OWNED)
        raise AssertionError(name)

    def direct_call(self, target: Direct, node: ast.Call) -> Value:
        """A call of a module level function bound at transpile time.

        Omitted parameters read their default from the function object
        (`m.default(..)`), after all arguments are evaluated, like CPython.
        Immutable literal defaults are written in place instead, unless the
        module assigns `__defaults__` or `__kwdefaults__` (design §6).
        """
        module = self.ctx.module
        fn = target.fn
        params = params_of(fn.args)
        n_args = len(node.args)
        values = self.operands([*node.args, *(k.value for k in node.keywords)])
        roles = ["value"] * len(values)
        order: list[int] = []
        per_slot: list[list[int]] = []
        for param, slot in zip(params, target.slots, strict=True):
            match slot.kind:
                case "arg" | "varargs":
                    indices = list(slot.indices)
                case "keyword" | "varkw":
                    indices = [n_args + j for j in slot.indices]
                case _:
                    default = default_of(fn.args, param)
                    if not module.bindings.writes_defaults and is_immutable_constant(default):
                        # `greet("Bob")` passes the literal default `"Hello"`.
                        assert isinstance(default, ast.Constant)
                        values.append(self.constant(default.value, default))
                    else:
                        read = self.module_call("default", rs.StrLit(fn.name), rs.StrLit(param))
                        values.append(Value(_try(read), Kind.OWNED))
                    roles.append(param)
                    indices = [len(values) - 1]
            for i in indices:
                roles[i] = param
            order += indices
            per_slot.append(indices)
        values = self.in_rust_order(values, order, roles)

        call_args: list[rs.Expr] = [self.module()]
        for slot, indices in zip(target.slots, per_slot, strict=True):
            used = [values[i] for i in indices]
            match slot.kind:
                case "varargs":
                    call_args.append(rs.Ref(self.tuple_of(used)))
                case "varkw":
                    names = [node.keywords[j].arg or "" for j in slot.indices]
                    call_args.append(rs.Ref(self.dict_of(names, used)))
                case _:
                    call_args.append(used[0].ref())
        call = rs.Call(rs.Path(module.fn_names[fn.name]), tuple(call_args))
        return Value(_try(call), Kind.OWNED)

    def tuple_of(self, values: list[Value]) -> rs.Expr:
        self.use("pyo3::types::PyTuple")
        if not values:
            return rs.Call(rs.Path("PyTuple::empty"), (self.py(),))
        array = rs.Array(tuple(self.elements(values)))
        return _try(rs.Call(rs.Path("PyTuple::new"), (self.py(), array)))

    def dict_of(self, names: list[str], values: list[Value]) -> rs.Expr:
        if not values:
            self.use("pyo3::types::PyDict")
            return rs.Call(rs.Path("PyDict::new"), (self.py(),))
        self.use("pyo3::types::IntoPyDict")
        pairs = rs.Array(
            tuple(
                rs.Tuple((rs.StrLit(name), v))
                for name, v in zip(names, self.elements(values), strict=True)
            )
        )
        return _try(rs.MethodCall(pairs, "into_py_dict", (self.py(),)))

    # --- function objects ---

    def function_value(self, node: FunctionNode, rust: str, bind: bool = False) -> Value:
        """The function object of a `def` or `lambda` whose Rust fn is `rust`:
        `rt::function!(..)` with the decorators applied, or `rt::def!(..)`
        with `bind`.

        Python evaluates the decorators, then the defaults, then applies the
        decorators bottom-up. Decorators are evaluated into temporaries and
        applied one statement at a time, which keeps that order and reads
        like the Python code:

            let decorator = m.global("functools")?.call_method1(..)?;
            let function = rt::function!(m, wrapper(*args, **kwargs))?;
            let wrapper = decorator.call1((function,))?;
        """
        from steelix.lower.function import function_options, signature_text

        scope = self.ctx.module.scopes.of(node)
        decorators = node.decorator_list if isinstance(node, ast.FunctionDef) else []
        decorator_values = [
            self.hoist(replace(self.value(d), base=None), "decorator") for d in decorators
        ]
        args = node.args
        defaults = [*args.defaults, *(d for d in args.kw_defaults if d is not None)]
        default_exprs = [v.arg() for v in self.operands(list(defaults))]
        names = self.comprehension_names()
        signature = signature_text(self.ctx, scope, rust, default_exprs, names)
        options = function_options(self.ctx, scope, rust)
        definer = self.module()
        if (
            self.ctx.class_body is not None
            and self.ctx.function is None
            and isinstance(node, ast.FunctionDef)
            and not self.ctx.module.future_annotations
        ):
            # A method: its annotations see the class namespace (`__classdict__`).
            definer = rs.Path(NAMESPACE)
        macro_args = (definer, signature, *(rs.Tokens((o,)) for o in options))
        macro = rs.MacroCall("rt::def" if bind else "rt::function", macro_args)
        value = Value(_try(macro), Kind.OWNED)
        if not decorator_values:
            return value
        value = self.hoist(value, "function")
        assert isinstance(value.expr, rs.Path)
        for decorator in reversed(decorator_values[1:]):
            applied = self.call_value(decorator, [value])
            self.prelude.append(rs.Let(value.expr.text, init=applied.expr))
        return self.call_value(decorator_values[0], [value])


_FRAME_BUILTINS = frozenset({"locals", "vars", "dir", "exec", "eval", "globals", "super"})
"""Builtins that use the namespace of the calling frame when called without one."""


def _reads_frame(name: str, node: ast.Call, shadowed: bool) -> bool:
    """`locals()`, `vars()`, `dir()`, or `exec` and `eval` without a namespace."""
    if shadowed:
        return False
    if name in ("locals", "vars", "dir"):
        return not node.args and not node.keywords
    if name in ("exec", "eval"):
        return len(node.args) + len(node.keywords) < 2
    return False


def keyword_arg(name: str) -> str:
    return _keyword(name)


def _keyword(name: str) -> str:
    """A keyword argument name in `rt::call!`: an identifier if possible."""
    rust = rust_ident(name)
    if rust.removeprefix("r#") == name:
        return rust
    return string_literal(name)


def _expr_base(expr: ast.expr) -> str | None:
    match expr:
        case ast.Name(id):
            return id
        case ast.Attribute(_, attr):
            return attr
    return None


def _all(conds: list[rs.Expr]) -> rs.Expr:
    """`a && b && c`."""
    result = conds[0]
    for cond in conds[1:]:
        result = rs.Binary("&&", result, cond)
    return result


def _if_expr(cond: rs.Expr, then: rs.Block, orelse: rs.Block) -> rs.IfExpr:
    # `else { if .. }` reads better as `else if ..`.
    if not orelse.stmts and isinstance(orelse.tail, rs.IfExpr):
        return rs.IfExpr(cond, then, orelse.tail)
    return rs.IfExpr(cond, then, orelse)


def _is_none_literal(value: Value) -> bool:
    return (
        value.kind is Kind.LITERAL
        and isinstance(value.expr, rs.MethodCall)
        and value.expr.method == "None"
    )


def _native_int_comparable(lhs: Value, rhs: Value) -> bool:
    """A Rust integer such as `len(xs)` against a non-negative int literal."""

    def literal(value: Value) -> bool:
        return (
            value.kind is Kind.LITERAL
            and isinstance(value.expr, rs.IntLit)
            and value.expr.value >= 0
            and not value.expr.suffix
        )

    return (lhs.kind is Kind.NATIVE and literal(rhs)) or (rhs.kind is Kind.NATIVE and literal(lhs))


def _literal_type(expr: rs.Expr) -> str | None:
    match expr:
        case rs.IntLit(_, suffix):
            return f"int{suffix}"
        case rs.FloatLit():
            return "float"
        case rs.StrLit():
            return "str"
        case rs.BoolLit():
            return "bool"
    return None


def _is_number(value: object) -> bool:
    return isinstance(value, int | float) and not isinstance(value, bool)


def _has_surrogates(value: str) -> bool:
    return any(0xD800 <= ord(ch) <= 0xDFFF for ch in value)
