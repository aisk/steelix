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

from steelix.frontend.resolve import Direct, Lookup, resolve_global
from steelix.lower.context import Context
from steelix.rustast import nodes as rs


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
    typed: bool = False
    """The value is a `Bound<'py, T>` of a concrete type such as `PyString`."""
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
                return self.expr
            case Kind.LITERAL | Kind.BOOL | Kind.NATIVE:
                assert self.owned_literal is not None
                return rs.Ref(self.owned_literal)
        if self.typed:
            # `&PyList::new(..)?` does not coerce: `?` needs the exact type.
            return rs.MethodCall(self.expr, "as_any")
        return rs.Ref(self.expr)

    def receiver(self) -> rs.Expr:
        """As the receiver of a PyO3 method call (auto-referenced)."""
        if self.kind in (Kind.LITERAL, Kind.BOOL, Kind.NATIVE):
            assert self.owned_literal is not None
            return self.owned_literal
        return self.expr

    def identity(self) -> rs.Expr:
        """As the argument of `Bound::is`, which takes `AsRef<Py<PyAny>>`."""
        if self.kind in (Kind.LITERAL, Kind.BOOL, Kind.NATIVE):
            return self.receiver()
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
        return _try(rs.MethodCall(self.receiver(), "is_truthy"))


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


class ExprLowering:
    def __init__(self, ctx: Context) -> None:
        self.ctx = ctx
        self.prelude: list[rs.Stmt] = []
        """Statements that must run before the expression being lowered."""

    def py(self) -> rs.Path:
        return rs.Path(self.ctx.py())

    def globals(self) -> rs.Path:
        return rs.Path(self.ctx.globals())

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
        stmt = rs.Let(name, init=value.expr)
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
                call = rs.MethodCall(inner.receiver(), _UNARY[type(op)])
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
            case ast.Call(func, args):
                return self.call(func, args, expr)
            case ast.Attribute(obj, attr):
                inner = self.value(obj)
                call = rs.MethodCall(inner.receiver(), "getattr", (rs.StrLit(attr),))
                return Value(_try(call), Kind.OWNED)
            case ast.Subscript(obj, index):
                container, key = self.operands([obj, index])
                call = rs.MethodCall(container.receiver(), "get_item", (key.arg(),))
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
        lhs, rhs = self.operands([left, right])
        args: tuple[rs.Expr, ...] = (rhs.arg(),)
        if method == "pow":
            args += (rs.MethodCall(self.py(), "None"),)
        return Value(_try(rs.MethodCall(lhs.receiver(), method, args)), Kind.OWNED)

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
                contains = _try(rs.MethodCall(rhs.receiver(), "contains", (lhs.arg(),)))
                return self.bool_value(contains if isinstance(op, ast.In) else negate(contains))
            case ast.Is() | ast.IsNot():
                cond = self._identity(lhs, rhs)
                pure = lhs.is_pure and rhs.is_pure
                return self.bool_value(cond if isinstance(op, ast.Is) else negate(cond), pure)
        self.use("pyo3::pyclass::CompareOp")
        method = _COMPARE[type(op)][1]
        call = rs.MethodCall(
            lhs.receiver(), "rich_compare", (rhs.arg(), rs.Path(f"CompareOp::{method}"))
        )
        return Value(_try(call), Kind.OWNED)

    def _identity(self, lhs: Value, rhs: Value) -> rs.Expr:
        if _is_none_literal(rhs):
            return rs.MethodCall(lhs.receiver(), "is_none")
        if _is_none_literal(lhs):
            return rs.MethodCall(rhs.receiver(), "is_none")
        return rs.MethodCall(lhs.receiver(), "is", (rhs.identity(),))

    def compare_condition(
        self, left: ast.expr, ops: list[ast.cmpop], comparators: list[ast.expr]
    ) -> rs.Expr:
        lhs, rhs = self.operands([left, comparators[0]])
        if len(ops) > 1 and not rhs.is_pure:
            lhs, rhs = self.hoist_all([lhs, rhs], ["lhs", "rhs"])
        result = self.compare_bool(ops[0], lhs, rhs)
        for op, comparator in zip(ops[1:], comparators[1:], strict=True):
            lhs = rhs
            with self.fresh() as pre:
                rhs = self.value(comparator)
                if comparator is not comparators[-1]:
                    rhs = self.hoist(rhs, "rhs")
                cond = self.compare_bool(op, lhs, rhs)
            if pre:
                cond = rs.BlockExpr(rs.Block(tuple(pre), cond))
            result = rs.Binary("&&", result, cond)
        return result

    def compare_bool(self, op: ast.cmpop, lhs: Value, rhs: Value) -> rs.Expr:
        """One comparison as a Rust `bool`, as in a condition."""
        if isinstance(op, ast.In | ast.NotIn | ast.Is | ast.IsNot):
            return self.compare_values(op, lhs, rhs).expr
        method, _, symbol = _COMPARE[type(op)]
        if _native_int_comparable(lhs, rhs):
            # `len(xs) > 3`: both sides are plain Rust integers.
            return rs.Binary(symbol, lhs.expr, rhs.expr)
        return _try(rs.MethodCall(lhs.receiver(), method, (rhs.arg(),)))

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
        return Value(_try(call), Kind.OWNED, typed=True, pure=pure)

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
            return Value(call, Kind.OWNED, typed=True)
        values = self.operands(list(elts))
        array = rs.Array(tuple(self.elements(values)))
        call = _try(rs.Call(rs.Path(f"{type_name}::new"), (self.py(), array)))
        return Value(call, Kind.OWNED, typed=True)

    def dict(self, keys: list[ast.expr], values: list[ast.expr]) -> Value:
        if not keys:
            self.use("pyo3::types::PyDict")
            return Value(rs.Call(rs.Path("PyDict::new"), (self.py(),)), Kind.OWNED, typed=True)
        self.use("pyo3::types::IntoPyDict")
        items = self.operands([e for pair in zip(keys, values, strict=True) for e in pair])
        lowered_keys = self.elements(items[0::2])
        lowered_values = self.elements(items[1::2])
        pairs = rs.Array(
            tuple(rs.Tuple((k, v)) for k, v in zip(lowered_keys, lowered_values, strict=True))
        )
        call = _try(rs.MethodCall(pairs, "into_py_dict", (self.py(),)))
        return Value(call, Kind.OWNED, typed=True)

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
                value = Value(_try(rs.MethodCall(value.receiver(), "str")), Kind.OWNED, typed=True)
            case "r":
                value = Value(_try(rs.MethodCall(value.receiver(), "repr")), Kind.OWNED, typed=True)
            case "a":
                value = Value(_try(_rt("ascii", value.ref())), Kind.OWNED, typed=True)
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

    def call(self, func: ast.expr, args: list[ast.expr], node: ast.Call) -> Value:
        if len(args) > MAX_CALL_ARGS:
            raise self.ctx.unsupported(f"call with more than {MAX_CALL_ARGS} arguments", node)
        if isinstance(func, ast.Name) and self.is_global(func.id):
            special = self.global_call(func.id, args, node)
            if special is not None:
                return special
        if isinstance(func, ast.Attribute):
            return self.method_call(func, args)
        callee, *values = self.operands([func, *args])
        return self.call_value(callee, values)

    def call_value(self, callee: Value, values: list[Value]) -> Value:
        if values:
            tuple_ = rs.Tuple(tuple(v.arg() for v in values))
            call = rs.MethodCall(callee.receiver(), "call1", (tuple_,))
        else:
            call = rs.MethodCall(callee.receiver(), "call0")
        return Value(_try(call), Kind.OWNED)

    def method_call(self, func: ast.Attribute, args: list[ast.expr]) -> Value:
        """`o.m(args)`: `call_method1` when that keeps Python's order, which
        looks the method up before evaluating the arguments."""
        obj = self.value(func.value)
        start = len(self.prelude)
        getattr_ = rs.MethodCall(obj.receiver(), "getattr", (rs.StrLit(func.attr),))
        method = Value(_try(getattr_), Kind.OWNED, base=func.attr)
        callee, *values = self.operands(list(args), [method])
        if len(self.prelude) == start and all(v.is_pure for v in values):
            name = rs.StrLit(func.attr)
            if values:
                tuple_ = rs.Tuple(tuple(v.arg() for v in values))
                call = rs.MethodCall(obj.receiver(), "call_method1", (name, tuple_))
            else:
                call = rs.MethodCall(obj.receiver(), "call_method0", (name,))
            return Value(_try(call), Kind.OWNED)
        return self.call_value(callee, values)

    def is_global(self, name: str) -> bool:
        fn = self.ctx.function
        return fn is None or not fn.scope.is_local(name)

    def global_call(self, name: str, args: list[ast.expr], node: ast.Call) -> Value | None:
        """Calls of module functions and builtins with a direct Rust spelling."""
        bindings = self.ctx.module.bindings
        if not bindings.is_module_bound(name) and not args:
            if name == "globals":
                owned = rs.MethodCall(rs.MethodCall(self.globals(), "clone"), "into_any")
                return Value(owned, Kind.OWNED)
            if name in ("locals", "vars"):
                # They read the calling frame, which transpiled code does not have.
                raise self.ctx.unsupported(f"{name}()", node)
        resolution = resolve_global(
            bindings, name, call_arity=len(args), module_index=self.ctx.statement_index
        )
        if isinstance(resolution, Direct):
            return self.direct_call(resolution.fn, args)
        if resolution is not Lookup.BUILTIN:
            return None
        if name == "print":
            values = self.operands(list(args))
            tuple_ = rs.Tuple(tuple(v.arg() for v in values))
            call = rs.Call(rs.Path("rt::builtins::print"), (self.py(), tuple_))
            return Value(_try(call), Kind.OWNED)
        if (name, len(args)) in _SPECIAL_BUILTINS:
            return self.builtin_call(name, self.operands(list(args)))
        return None

    def builtin_call(self, name: str, args: list[Value]) -> Value:
        """Builtins whose PyO3 method does exactly what the builtin does."""
        arg = args[0]
        match name:
            case "len":
                length = _try(rs.MethodCall(arg.receiver(), "len"))
                return Value(length, Kind.NATIVE, _rt("int", self.py(), length), base="len")
            case "isinstance":
                check = _try(rs.MethodCall(arg.receiver(), "is_instance", (args[1].ref(),)))
                return self.bool_value(check)
            case "bool":
                return self.bool_value(arg.truthy())
            case "type":
                return Value(rs.MethodCall(arg.receiver(), "get_type"), Kind.OWNED, typed=True)
            case "iter":
                call = _try(rs.MethodCall(arg.receiver(), "try_iter"))
                return Value(call, Kind.OWNED, typed=True)
            case "str" | "repr":
                call = _try(rs.MethodCall(arg.receiver(), name))
                return Value(call, Kind.OWNED, typed=True)
            case "abs":
                return Value(_try(rs.MethodCall(arg.receiver(), "abs")), Kind.OWNED)
        raise AssertionError(name)

    def direct_call(self, target: ast.FunctionDef, args: list[ast.expr]) -> Value:
        module = self.ctx.module
        call_args: list[rs.Expr] = [self.py()]
        if target in module.callgraph.needs_globals:
            call_args.append(self.globals())
        call_args += [v.ref() for v in self.operands(list(args))]
        call = rs.Call(rs.Path(module.fn_names[target.name]), tuple(call_args))
        return Value(_try(call), Kind.OWNED)


def _expr_base(expr: ast.expr) -> str | None:
    match expr:
        case ast.Name(id):
            return id
        case ast.Attribute(_, attr):
            return attr
    return None


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
