"""Create the function objects of `def` and `lambda`: `rt::function!` with
the decorators and defaults."""

from __future__ import annotations

import ast
from dataclasses import replace
from typing import TYPE_CHECKING

from steelix.frontend.scope import FunctionNode
from steelix.lower.context import NAMESPACE
from steelix.lower.function import defined_in_frame, function_options, signature_text
from steelix.lower.names import rust_ident
from steelix.lower.rustflow import fallible
from steelix.lower.values import Kind, Value
from steelix.rustast import nodes as rs

if TYPE_CHECKING:
    from steelix.lower.expr import ExprLowering


def _try(expr: rs.Expr) -> rs.Expr:
    return rs.Try(expr)


class FunctionObjectLowering:
    """The part of `ExprLowering` for function objects."""

    def function_value(self: ExprLowering, node: FunctionNode, rust: str) -> Value:
        """The function object of a `def` or `lambda` whose Rust fn is `rust`,
        with the decorators applied: `rt::function!(..)`.

        Python evaluates the decorators, then the defaults, then applies the
        decorators bottom-up. The macro does the same with decorators written
        before the name, as in Python:

            rt::function!(m, @(m.global("functools")?.method1("wraps", (f,))?) wrapper(*args))?

        When a decorator or a default needs statements of its own (a
        prelude), which would run before the decorators written in the macro,
        or when one may assign a local that another one reads (`:=`), the
        decorators are evaluated into temporaries and applied one statement
        at a time instead:

            let decorator = ..;
            let function = rt::function!(m, wrapper(*args, **kwargs))?;
            decorator.call1((function,))?

        So does `--traceback-lines` when CPython would report more than one
        line: each decorator is evaluated and applied on its own line, the
        defaults on the line of `def`, and only the separate statements can
        have a `frame.line(n)` each. The statement starts on the line of the
        first decorator (`self.line`).
        """
        scope = self.ctx.module.scopes.of(node)
        decorators = node.decorator_list if isinstance(node, ast.FunctionDef) else []
        lowered: list[tuple[Value, list[rs.Stmt]]] = []
        for decorator in decorators:
            with self.fresh() as prelude:
                value = replace(self.value(decorator), base=None)
            lowered.append((value, prelude))
        args = node.args
        defaults = [*args.defaults, *(d for d in args.kw_defaults if d is not None)]
        with self.fresh() as defaults_prelude:
            default_exprs = [v.arg() for v in self.operands(list(defaults))]
        defaults_fallible = fallible([*defaults_prelude, *map(rs.ExprStmt, default_exprs)])
        inline = not any(prelude for _, prelude in lowered) and not (
            decorators and defaults_prelude
        )
        if decorators and any(
            isinstance(n, ast.NamedExpr) for e in [*decorators, *defaults] for n in ast.walk(e)
        ):
            inline = False
        if self.ctx.module.traceback_lines and (len(decorators) > 1 or defaults_fallible):
            inline = False
        definer = self.module()
        if (
            self.ctx.class_body is not None
            and self.ctx.function is None
            and isinstance(node, ast.FunctionDef)
        ):
            # A method: its qualified name and annotations know the class.
            definer = rs.Path(NAMESPACE)
        prefixes: list[rs.Expr] = []
        temps: list[Value] = []
        for decorator, (value, prelude) in zip(decorators, lowered, strict=True):
            if inline:
                prefixes.append(self._decorator(value, definer))
            else:
                if fallible([*prelude, rs.ExprStmt(value.expr)]):
                    self.prelude += self.at_line(decorator.lineno)
                self.prelude += prelude
                temps.append(self.hoist(value, "decorator"))
        if temps and defaults_fallible:
            self.prelude += self.at_line(node.lineno)
        self.prelude += defaults_prelude
        names = self.comprehension_names()
        options = function_options(self.ctx, scope, rust, definer)
        # A `qualname = ..` option gives the name too (its last part).
        named = any(str(o.parts[0]).startswith("qualname") for o in options)
        signature = signature_text(self.ctx, scope, rust, default_exprs, names, named)
        # In a function body the frame is the definer, which knows the
        # qualified name; decorator names are still read from the module.
        where = rs.Path("frame") if defined_in_frame(self.ctx, scope, definer) else definer
        macro = rs.MacroCall("rt::function", (where, *prefixes, signature, *options))
        value = Value(_try(macro), Kind.OWNED)
        if not temps:
            return value
        value = self.hoist(value, "function")
        assert isinstance(value.expr, rs.Path)
        for decorator, source in reversed(list(zip(temps[1:], decorators[1:], strict=True))):
            self.prelude += self.at_line(source.lineno)
            applied = self.call_value(decorator, [value])
            self.prelude.append(rs.Let(value.expr.text, init=applied.expr))
        self.prelude += self.at_line(decorators[0].lineno)
        return self.call_value(temps[0], [value])

    def _decorator(self: ExprLowering, value: Value, definer: rs.Expr) -> rs.Decorator:
        """A decorator as written before the name in `rt::function!`:
        `@name.attr` for a name read where the function is defined (the
        definer's `get` or `global`), else `@(expr)`."""
        lookup = "get" if definer == rs.Path(NAMESPACE) else "global"
        attrs: list[str] = []
        expr = value.expr
        while value.kind is Kind.OWNED:
            match expr:
                case rs.Try(rs.MethodCall(inner, "attr", (rs.StrLit(attr),))) if _plain(attr):
                    attrs.append(attr)
                    expr = inner
                    continue
                case rs.Try(rs.MethodCall(receiver, method, (rs.StrLit(name),))) if (
                    receiver == definer and method == lookup and _plain(name)
                ):
                    return rs.Decorator((".".join(["@" + name, *reversed(attrs)]),))
            break
        match value.kind:
            case Kind.PLACE:
                expr = rs.Ref(value.expr)
            case Kind.LITERAL | Kind.BOOL | Kind.NATIVE:
                assert value.owned_literal is not None
                expr = value.owned_literal
            case _:
                expr = value.expr
        return rs.Decorator(("@(", expr, ")"))


def _plain(name: str) -> bool:
    """Whether `name` is spelled the same as a Rust identifier, which the
    macro turns back into the name with `stringify!`."""
    return name != "_" and rust_ident(name) == name
