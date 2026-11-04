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
from dataclasses import replace

from steelix.frontend.resolve import Lookup, resolve_global
from steelix.lower.calls import CallLowering
from steelix.lower.comprehension import ComprehensionLowering, ComprehensionVariable
from steelix.lower.context import NAMESPACE, Context
from steelix.lower.defs import FunctionObjectLowering
from steelix.lower.values import MAX_CALL_ARGS, Kind, Operand, Value
from steelix.rustast import nodes as rs

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


class ExprLowering(CallLowering, FunctionObjectLowering, ComprehensionLowering):
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
        if value.kind in (Kind.BOOL, Kind.NATIVE, Kind.REF):
            self.ctx.pinned.add(name)  # holds no object of its own
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
            case ast.Await(operand):
                return self.suspension("await_", operand, "awaitable")
            case ast.Yield(operand):
                return self.suspension("yield_", operand, "value")
            case ast.YieldFrom(operand):
                return self.suspension("yield_from", operand, "iterable")
            case ast.Lambda():
                from steelix.lower.function import lower_nested

                return self.function_value(expr, lower_nested(self.ctx, expr))
            case ast.Attribute(obj, attr):
                inner = self.value(obj)
                call = inner.method("attr", rs.StrLit(attr))
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
                return self.dict(keys, values)
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
                if not pre_then and not pre_else and isinstance(then, rs.BoolLit):
                    # `True if c else False` is `c`.
                    if then == rs.BoolLit(True) and other == rs.BoolLit(False):
                        return cond
                    if then == rs.BoolLit(False) and other == rs.BoolLit(True):
                        return negate(cond)
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
            case complex():
                parts = (rs.FloatLit(value.real), rs.FloatLit(value.imag))
                return Value(_rt("complex", py(), *parts), Kind.OWNED, pure=True)
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
            if name in fn.bool_locals:
                return self.bool_value(rs.Path(rust), pure=True)
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
        if values and all(v.kind is Kind.BOOL or _is_bool_literal(v) for v in values):
            return [v.expr for v in values]  # Rust `bool`s: `[found, x.is_none()]`
        return [v.ref() for v in values]

    def sequence(self, type_name: str, elts: list[ast.expr]) -> Value:
        """A list, tuple or set display."""
        if any(isinstance(elt, ast.Starred) for elt in elts):
            return self.unpacking_display(type_name, elts)
        self.use(f"pyo3::types::{type_name}")
        if not elts and type_name != "PySet":
            call: rs.Expr = rs.Call(rs.Path(f"{type_name}::empty"), (self.py(),))
            return Value(call, Kind.OWNED, typed=type_name)
        values = self.operands(list(elts))
        array = rs.Array(tuple(self.elements(values)))
        call = _try(rs.Call(rs.Path(f"{type_name}::new"), (self.py(), array)))
        return Value(call, Kind.OWNED, typed=type_name)

    def unpacking_display(self, type_name: str, elts: list[ast.expr]) -> Value:
        """`[a, *b]`, `(*a,)`, `{*a, b}`: `rt::list!(py, a, *b)?` and so on,
        which evaluates the elements in order and iterates each `*` value
        right away, like CPython. An element that needs statements first is
        a block of its own, so that they run at its turn."""
        parts: list[rs.Expr] = []
        for elt in elts:
            if isinstance(elt, ast.Starred):
                parts.append(rs.Tokens(("*", self.display_part(elt.value, star=True))))
            else:
                parts.append(self.display_part(elt))
        macro = {"PyList": "rt::list", "PyTuple": "rt::tuple", "PySet": "rt::set"}[type_name]
        call = _try(rs.MacroCall(macro, (self.py(), *parts)))
        return Value(call, Kind.OWNED, typed=type_name)

    def display_part(self, expr: ast.expr, star: bool = False) -> rs.Expr:
        """An element of `rt::list!` and the like: an `impl IntoPyObject`,
        or with `star`, an expression the macro borrows."""
        with self.fresh() as prelude:
            value = self.value(expr)
        if prelude:
            return rs.BlockExpr(rs.Block(tuple(prelude), value.owned()))
        return value.bound() if star else value.arg()

    def dict(self, keys: list[ast.expr | None], values: list[ast.expr]) -> Value:
        if any(key is None for key in keys):
            return self.unpacking_dict(keys, values)
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

    def unpacking_dict(self, keys: list[ast.expr | None], values: list[ast.expr]) -> Value:
        """`{**a, "k": v}`: `rt::dict!(py, **a, ("k", v))?`, merging each
        `**` mapping right away, like CPython."""
        parts: list[rs.Expr] = []
        for key, value in zip(keys, values, strict=True):
            if key is None:
                parts.append(rs.Tokens(("**", self.display_part(value, star=True))))
            else:
                parts.append(rs.Tuple((self.display_part(key), self.display_part(value))))
        call = _try(rs.MacroCall("rt::dict", (self.py(), *parts)))
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

    # --- await and yield ---

    def suspension(self, helper: str, operand: ast.expr | None, role: str) -> Value:
        """`rt::await_(x).await?`, `rt::yield_(&x).await?` or
        `rt::yield_from(x).await?` (design §8): the value is the result of
        the awaitable, what `send()` sends, or the delegate's return value.

        The future holds what the Rust statement holds when it suspends, and
        the GC sees neither. CPython is done with the parts of the operand
        once it suspends, so an operand whose Rust expression leaves
        temporaries (`m.global("helper")?` in `m.global("helper")?.call0()?`)
        is bound with `let` first: `let helper = ..; rt::await_(helper)`."""
        if operand is None:
            arg: rs.Expr = _rt("none", self.py())
        else:
            value = self.value(operand)
            if value.awaited and helper != "yield_":
                return replace(value, awaited=False)
            arg = self.suspended_operand(value, role, operand)
        future = rs.Await(_rt(helper, arg))
        return Value(_try(future), Kind.OWNED)

    def suspended_operand(self, value: Value, role: str, operand: ast.expr) -> rs.Expr:
        """The argument of `rt::await_` and the like, which take an owned
        or a borrowed `Bound`."""
        match value.kind:
            case Kind.LITERAL | Kind.BOOL | Kind.NATIVE:
                assert value.owned_literal is not None
                return value.owned_literal
            case Kind.PLACE:
                # A temporary of the prelude moves in.
                return value.expr if value.temp else rs.Ref(value.expr)
            case Kind.REF:
                return value.expr
        if not _leaves_temporaries(value.expr):
            return value.expr
        base = _suspension_base(operand)
        name = self.ctx.temp(base or role)
        if base is not None and name != base:
            # `sleep2` says less than `awaitable`.
            self.ctx.release(name)
            name = self.ctx.temp(role)
        self.prelude.append(rs.Let(name, init=value.expr))
        return rs.Path(name)


def _leaves_temporaries(expr: rs.Expr) -> bool:
    """Whether evaluating `expr` leaves Rust temporaries that live until the
    end of the statement: a method called on the result of a call, a borrow
    of a value that is not a variable, anything a macro builds. A value
    moved into a call or a tuple is no temporary."""
    match expr:
        case rs.Path() | rs.IntLit() | rs.FloatLit() | rs.StrLit() | rs.BoolLit() | rs.ByteStrLit():
            return False
        case rs.Try(inner):
            return _leaves_temporaries(inner)
        case rs.Ref(inner):
            return not isinstance(inner, rs.Path)
        case rs.MethodCall(receiver, _, args):
            if not isinstance(receiver, rs.Path):
                return True
            return any(_leaves_temporaries(a) for a in args)
        case rs.Call(_, args) | rs.Tuple(args) | rs.Array(args):
            return any(_leaves_temporaries(a) for a in args)
    return True


def _suspension_base(operand: ast.expr) -> str | None:
    """A name for the temporary holding what is awaited: the called
    function's (`sleep` for `asyncio.sleep(1)`)."""
    match operand:
        case ast.Call(ast.Name(name) | ast.Attribute(attr=name)):
            return name.lower()
    return None


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


def _is_bool_literal(value: Value) -> bool:
    return value.kind is Kind.LITERAL and isinstance(value.expr, rs.BoolLit)


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
