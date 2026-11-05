"""Lower comprehensions, generator expressions and `:=` (design §8).

A list, set or dict comprehension runs inline, like CPython since 3.12
(PEP 709): its loops go into the prelude of the statement, building the
container in a temporary, and its variables are `let` bindings in the loop
body, so they do not leak and shadow nothing outside:

    let list = PyList::empty(py);
    for x in xs.try_iter()? {
        let x = x?;
        if x.is_truthy()? {
            list.append(f(m, &x)?)?;
        }
        py.check_signals()?;
    }

A generator expression is a nested fn, the *step*, holding its `for` loop
(`function.lower_genexpr`); `rt::genexpr!` creates the generator object from
it and from the iterator of the first iterable, which is evaluated right away.

`:=` assigns the variable in the prelude and then reads it.
"""

from __future__ import annotations

import ast
from collections.abc import Callable
from dataclasses import dataclass
from typing import TYPE_CHECKING

from steelix.frontend.scope import FunctionNode, comprehension_variables
from steelix.lower.names import FUNCTION_RESERVED, rust_ident
from steelix.lower.values import Kind, Value
from steelix.rustast import nodes as rs
from steelix.rustast.printer import string_literal

if TYPE_CHECKING:
    from steelix.lower.expr import ExprLowering


Comprehension = ast.ListComp | ast.SetComp | ast.DictComp

# The container a comprehension builds: its PyO3 type, the base of the
# temporary's name, and how an item is added.
_CONTAINERS: dict[type[ast.expr], tuple[str, str]] = {
    ast.ListComp: ("PyList", "list"),
    ast.SetComp: ("PySet", "set"),
    ast.DictComp: ("PyDict", "dict"),
}


@dataclass(frozen=True)
class ComprehensionVariable:
    """A variable of a comprehension (or of the generator expression whose
    step is being lowered), declared in its loop body."""

    rust: str
    cell: bool = False
    """Captured by a function, lambda or generator expression defined in the
    comprehension: an `rt::Cell` created before the loop."""


def line_marker(line: int) -> rs.Stmt:
    return rs.ExprStmt(rs.MethodCall(rs.Path("frame"), "line", (rs.IntLit(line),)))


def unpack_pattern(names: list[str], star: int | None) -> str:
    """`[a, b]`, or `([a], rest, [b])` for `rt::unpack_star`."""
    if star is None:
        return f"[{', '.join(names)}]"
    before = ", ".join(names[:star])
    after = ", ".join(names[star + 1 :])
    return f"([{before}], {names[star]}, [{after}])"


class ComprehensionLowering:
    """The part of `ExprLowering` for comprehensions, generator expressions
    and `:=`."""

    def at_line(self: ExprLowering, line: int) -> list[rs.Stmt]:
        """`frame.line(line);` unless the frame is known to be there.

        Statements set the line of the frame (`StmtLowering.at_line`);
        expressions only do inside comprehensions, whose parts CPython
        reports on their own lines, and put the line back at their end.
        """
        if self.line == line:
            return []
        self.line = line
        self.markers += 1
        return [line_marker(line)] if self.ctx.module.traceback_lines else []

    def comprehension_variable(self: ExprLowering, name: str) -> ComprehensionVariable | None:
        for variables in reversed(self.comprehensions):
            if name in variables:
                return variables[name]
        return None

    def declare_variables(
        self: ExprLowering, node: Comprehension | ast.GeneratorExp
    ) -> dict[str, ComprehensionVariable]:
        """Rust names for the variables of a comprehension. In a function
        they are the names of the function's renamer (`symtable` counts them
        as its locals), so they can not clash with anything the comprehension
        refers to; temporaries avoid them too."""
        cells = self.captured_variables(node)
        fn = self.ctx.function
        variables = {}
        for name in sorted(comprehension_variables(node)):
            cell = name in cells
            if fn is not None and name in fn.names and not cell:
                rust = fn.names[name]
            else:
                # A cell outlives the loop: its name must not hide a local.
                rust = rust_ident(name)
                taken = FUNCTION_RESERVED | {self.ctx.context(), "ns"} | self.ctx.item_names
                taken |= set(self.ctx.module.fn_names.mapping.values())
                if fn is not None:
                    taken |= set(fn.names.mapping.values())
                while rust in taken or rust in self.ctx.taken_temps():
                    rust += "_"
            variables[name] = ComprehensionVariable(rust, cell)
        return variables

    def captured_variables(self: ExprLowering, node: Comprehension | ast.GeneratorExp) -> set[str]:
        """Variables of the comprehension that functions defined in it capture."""
        scopes = self.ctx.module.scopes.functions
        names = comprehension_variables(node)
        found: set[str] = set()
        for child in ast.walk(node):
            if child is not node and isinstance(child, FunctionNode) and child in scopes:
                found |= names & scopes[child].comprehension_frees
        return found

    def comprehension_names(self: ExprLowering) -> dict[str, str]:
        """The Rust names of the comprehension variables in scope, which
        nested functions capture (as cells)."""
        names: dict[str, str] = {}
        for variables in self.comprehensions:
            names.update({name: var.rust for name, var in variables.items()})
        return names

    # --- list, set and dict comprehensions ---

    def comprehension(self: ExprLowering, node: Comprehension) -> Value:
        type_name, base = _CONTAINERS[type(node)]
        self.use(f"pyo3::types::{type_name}")
        entry_line = self.line
        first = node.generators[0]
        # The iterable of the first `for` is evaluated in the enclosing scope.
        self.prelude += self.at_line(first.iter.lineno)
        iterable = self.value(first.iter)
        variables = self.declare_variables(node)
        py = rs.Path(self.ctx.py())
        # Temporaries in the loop must not hide the variables.
        variables_by_rust = {v.rust: v for v in variables.values()}
        reserved = [v.rust for v in variables.values() if v.rust not in self.ctx.taken_temps()]
        for name, var in variables.items():
            self.ctx.reserve(var.rust)
            if var.cell:
                empty = rs.Call(rs.Path("rt::Cell::empty"), (py, rs.StrLit(name)))
                self.prelude.append(rs.Let(var.rust, init=rs.Try(empty)))
        result = self.ctx.temp(base)
        match type_name:
            case "PyList":
                empty: rs.Expr = rs.Call(rs.Path("PyList::empty"), (py,))
            case "PySet":
                empty = rs.Try(rs.Call(rs.Path("PySet::empty"), (py,)))
            case _:
                empty = rs.Call(rs.Path("PyDict::new"), (py,))
        self.prelude.append(rs.Let(result, init=empty))
        self.comprehensions.append(variables)
        try:
            self.prelude += self.comprehension_loop(
                node, 0, iterable, lambda: self.add_item(node, rs.Path(result))
            )
        finally:
            self.comprehensions.pop()
            for name in reserved:
                if not variables_by_rust[name].cell:
                    # The loop's variables are gone after it.
                    self.ctx.release(name)
        if entry_line is not None:
            self.prelude += self.at_line(entry_line)
        return Value(rs.Path(result), Kind.PLACE, typed=type_name, temp=True)

    def add_item(self: ExprLowering, node: Comprehension, result: rs.Path) -> list[rs.Stmt]:
        """`list.append(x)?;`, `set.add(x)?;` or `dict.set_item(k, v)?;` with
        the statements evaluating the item."""
        with self.fresh() as prelude:
            if isinstance(node, ast.DictComp):
                key_line, value_line = node.key.lineno, node.value.lineno
                prelude += self.at_line(key_line)

                def value() -> Value:
                    self.prelude += self.at_line(value_line)
                    return self.value(node.value)

                key, item = self.operands([node.key, value])
                call = rs.MethodCall(result, "set_item", (key.arg(), item.arg()))
            else:
                prelude += self.at_line(node.elt.lineno)
                item = self.value(node.elt)
                method = "append" if isinstance(node, ast.ListComp) else "add"
                # `PySet::add` inserts, unlike `PyAnyMethods::add` (`+`).
                call = rs.MethodCall(result, method, (item.arg(),), concrete=True)
        return [*prelude, rs.ExprStmt(rs.Try(call))]

    def comprehension_loop(
        self: ExprLowering,
        node: Comprehension | ast.GeneratorExp,
        index: int,
        iterable: Value | rs.Path,
        innermost: Callable[[], list[rs.Stmt]],
    ) -> list[rs.Stmt]:
        """The `for` loop of the generator `index` with the loops of the
        following ones inside, and `innermost` in the innermost body.

        `iterable` is the lowered iterable, or the path of an iterator (the
        parameter of a generator expression's step), which `rt::iter`
        iterates, checking for signals before each next item. The loop sets
        the frame back to the line of the iterable whenever its body moved
        it, since getting the next item can raise."""
        generator = node.generators[index]
        iter_line = generator.iter.lineno
        out: list[rs.Stmt] = []
        # Like a `for` statement's (`StmtLowering.for_iterator`): with
        # `--traceback-lines` the body checks for signals itself.
        traceback_lines = self.ctx.module.traceback_lines
        if isinstance(iterable, rs.Path):
            # Already an iterator.
            iterator: rs.Expr = iterable
            if not traceback_lines:
                iterator = rs.Try(rs.Call(rs.Path("rt::iter"), (iterable,)))
        elif traceback_lines:
            iterator = rs.Try(iterable.method("try_iter"))
        else:
            iterator = rs.Try(rs.Call(rs.Path("rt::iter"), (iterable.ref(),)))
        markers = self.markers
        pattern, body = self.bind_target(generator.target)
        conditions: list[tuple[list[rs.Stmt], rs.Expr]] = []
        for test in generator.ifs:
            with self.fresh() as pre:
                pre += self.at_line(test.lineno)
                cond = self.condition(test)
            conditions.append((pre, cond))
        if index + 1 < len(node.generators):
            inner = node.generators[index + 1]
            with self.fresh() as pre:
                pre += self.at_line(inner.iter.lineno)
                inner_iterable = self.value(inner.iter)
            innermost_stmts = pre + self.comprehension_loop(
                node, index + 1, inner_iterable, innermost
            )
        else:
            innermost_stmts = innermost()
        body += _guarded(conditions, innermost_stmts)
        if not _returns(body):
            if traceback_lines:
                check = rs.MethodCall(rs.Path(self.ctx.py()), "check_signals")
                body.append(rs.ExprStmt(rs.Try(check)))
            if self.markers != markers:
                self.line = None
                body += self.at_line(iter_line)
        self.line = iter_line
        out.append(rs.For(pattern, iterator, rs.Block(tuple(body))))
        return out

    def bind_target(self: ExprLowering, target: ast.expr) -> tuple[str, list[rs.Stmt]]:
        """The pattern of a comprehension's `for` loop and the statements at
        the start of its body binding the target."""
        if isinstance(target, ast.Name):
            var = self.comprehension_variable(target.id)
            assert var is not None
            if not var.cell:
                # `for x in xs.try_iter()? { let x = x?; .. }`
                return var.rust, [rs.Let(var.rust, init=rs.Try(rs.Path(var.rust)))]
        item = self.ctx.temp("item")
        return item, self.bind(target, Value(rs.Try(rs.Path(item)), Kind.OWNED))

    def bind_next(self: ExprLowering, target: ast.expr) -> tuple[str, list[rs.Stmt]]:
        """`bind_target` for an item that is already a `Bound`, not a
        `PyResult`: the pattern of `if let Some(pattern) = rt::next(..)?`."""
        if isinstance(target, ast.Name):
            var = self.comprehension_variable(target.id)
            assert var is not None
            if not var.cell:
                return var.rust, []
        item = self.ctx.temp("item")
        return item, self.bind(target, Value(rs.Path(item), Kind.PLACE, temp=True))

    def bind(self: ExprLowering, target: ast.expr, value: Value) -> list[rs.Stmt]:
        """Assign `value` to a target of a comprehension's `for`."""
        match target:
            case ast.Name(id):
                var = self.comprehension_variable(id)
                assert var is not None
                if var.cell:
                    call = rs.MethodCall(rs.Path(var.rust), "set", (value.arg(),))
                    return [rs.ExprStmt(rs.Try(call))]
                return [rs.Let(var.rust, init=value.owned())]
            case ast.Tuple(elts) | ast.List(elts):
                star = next((i for i, e in enumerate(elts) if isinstance(e, ast.Starred)), None)
                helper = "rt::unpack" if star is None else "rt::unpack_star"
                call = rs.Try(rs.Call(rs.Path(helper), (value.ref(),)))
                targets = [e.value if isinstance(e, ast.Starred) else e for e in elts]
                plain = [
                    self.comprehension_variable(t.id) if isinstance(t, ast.Name) else None
                    for t in targets
                ]
                if all(v is not None and not v.cell for v in plain) and len(
                    {v.rust for v in plain if v is not None}
                ) == len(plain):
                    # `let [key, value] = rt::unpack(&item?)?;`
                    names = [v.rust for v in plain if v is not None]
                    return [rs.Let(unpack_pattern(names, star), init=call)]
                temps = [self.ctx.temp(_target_base(t)) for t in targets]
                out: list[rs.Stmt] = [rs.Let(unpack_pattern(temps, star), init=call)]
                for t, temp in zip(targets, temps, strict=True):
                    out += self.bind(t, Value(rs.Path(temp), Kind.OWNED, pure=True))
                return out
            case ast.Attribute(obj, attr):
                with self.fresh() as prelude:
                    o = self.value(obj)
                    call = o.method("set_attr", rs.StrLit(attr), value.arg())
                return [*prelude, rs.ExprStmt(rs.Try(call))]
            case ast.Subscript(obj, index):
                with self.fresh() as prelude:
                    value = self.hoist(value, "value")
                    o, i = self.operands([obj, index])
                    call = o.method("set_item", i.arg(), value.arg())
                return [*prelude, rs.ExprStmt(rs.Try(call))]
        raise self.ctx.unsupported(type(target).__name__, target)

    # --- generator expressions ---

    def generator_expression(self: ExprLowering, node: ast.GeneratorExp) -> Value:
        """`rt::genexpr!(m, genexpr[captures](xs.try_iter()?))`, after the
        step fn item (design §8)."""
        from steelix.lower.classes import merge_text
        from steelix.lower.function import captures_text, lower_genexpr

        entry_line = self.line
        first = node.generators[0]
        self.prelude += self.at_line(first.iter.lineno)
        iterable = self.value(first.iter)
        iterator = rs.Try(iterable.method("try_iter"))
        if entry_line is not None and entry_line != self.line:
            # The generator is created on the line of the expression.
            iterator_value = self.hoist(Value(iterator, Kind.OWNED, typed="PyIterator"), "iter")
            iterator = iterator_value.expr
            self.prelude += self.at_line(entry_line)
        rust = lower_genexpr(self.ctx, node, self.comprehension_names())
        scope = self.ctx.module.scopes.of(node)
        parts: list[str | rs.Expr] = [rs.Path(rust)]
        parts += captures_text(self.ctx, scope, self.comprehension_names())
        parts += ["(", iterator, ")"]
        args: list[rs.Expr] = [self.module(), rs.Tokens(merge_text(parts))]
        if scope.qualname != "<genexpr>":
            args.append(rs.Tokens((f"qualname = {string_literal(scope.qualname)}",)))
        return Value(rs.Try(rs.MacroCall("rt::genexpr", tuple(args))), Kind.OWNED)

    # --- assignment expressions ---

    def named_expr(self: ExprLowering, node: ast.NamedExpr) -> Value:
        """`(name := value)`: assign in the prelude, then read the variable.

        Reads of a local that `:=` assigns somewhere in the function are not
        pure (`ExprLowering.name`), so an earlier read in the same expression
        is evaluated into a temporary before the assignment."""
        assert isinstance(node.target, ast.Name)
        name = node.target.id
        value = self.value(node.value)
        fn = self.ctx.function
        if fn is None or fn.scope.is_global(name):
            value = self.hoist(value, name)
            store = self.scope_call("set", rs.StrLit(name), value.arg())
            self.prelude.append(rs.ExprStmt(rs.Try(store)))
            return value
        rust = fn.names[name]
        if fn.is_cell(name):
            value = self.hoist(value, name)
            store = rs.MethodCall(rs.Path(rust), "set", (value.arg(),))
            self.prelude.append(rs.ExprStmt(rs.Try(store)))
            return value
        if name in fn.locals.optional:
            value = self.hoist(value, name)
            # The value is used again: a temporary is cloned, not moved.
            stored = rs.MethodCall(value.expr, "clone") if value.kind is Kind.PLACE else None
            some = rs.Call(rs.Path("Some"), (stored or value.owned(),))
            self.prelude.append(rs.Assign(rs.Path(rust), some))
            return value
        if fn.locals.declared_at.get(name) is node:
            # The first assignment of the local, which always runs.
            mutable = name in fn.locals.mutable
            self.prelude.append(rs.Let(rust, mutable, init=value.owned()))
        else:
            self.prelude.append(rs.Assign(rs.Path(rust), value.owned()))
        return Value(rs.Path(rust), Kind.PLACE, pure=False, base=name)


def _guarded(conditions: list[tuple[list[rs.Stmt], rs.Expr]], body: list[rs.Stmt]) -> list[rs.Stmt]:
    """`body` run only if the `if` clauses of a comprehension hold, one
    `if` per clause, or `a && b` when the second needs no statements."""
    stmts = body
    for pre, cond in reversed(conditions):
        if len(stmts) == 1 and isinstance(stmts[0], rs.If) and stmts[0].orelse is None:
            inner = stmts[0]
            if inner.pattern is None:
                stmts = [*pre, rs.If(rs.Binary("&&", cond, inner.cond), inner.then)]
                continue
        stmts = [*pre, rs.If(cond, rs.Block(tuple(stmts)))]
    return stmts


def _returns(body: list[rs.Stmt]) -> bool:
    return bool(body) and isinstance(body[-1], rs.Return)


def _target_base(target: ast.expr) -> str:
    match target:
        case ast.Name(id):
            return id
        case ast.Attribute(_, attr):
            return attr
    return "item"
