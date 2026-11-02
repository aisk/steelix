"""Reject Python constructs that have no lowering yet.

Running this before any analysis keeps the analyses and the lowering simple:
they only ever see the supported subset, and every unsupported construct is
reported with its source location instead of producing wrong code.
"""

from __future__ import annotations

import ast

from steelix.diagnostics import SourceLocation, UnsupportedError

# Human readable names for the error message, by AST node type.
_FEATURES: dict[type[ast.AST], str] = {
    ast.AsyncFor: "async for",
    ast.AnnAssign: "annotated assignment",
    ast.Try: "try statement",
    ast.TryStar: "try/except*",
    ast.Raise: "raise",
    ast.With: "with statement",
    ast.AsyncWith: "async with",
    ast.ClassDef: "class definition",
    ast.AsyncFunctionDef: "async function",
    ast.Match: "match statement",
    ast.TypeAlias: "type alias",
    ast.NamedExpr: "assignment expression",
    ast.Slice: "slice outside a subscript",
    ast.Starred: "starred expression",
    ast.ListComp: "list comprehension",
    ast.SetComp: "set comprehension",
    ast.DictComp: "dict comprehension",
    ast.GeneratorExp: "generator expression",
    ast.Await: "await",
    ast.Yield: "yield",
    ast.YieldFrom: "yield from",
    ast.TemplateStr: "t-string",
}


class SubsetChecker:
    def __init__(self, filename: str) -> None:
        self.filename = filename
        self.depth = 0
        """Number of enclosing functions."""

    def fail(self, feature: str, node: ast.AST) -> UnsupportedError:
        return UnsupportedError(feature, SourceLocation.of(self.filename, node))

    def unsupported(self, node: ast.AST) -> UnsupportedError:
        feature = _FEATURES.get(type(node), type(node).__name__)
        return self.fail(feature, node)

    def check_module(self, module: ast.Module) -> None:
        for stmt in module.body:
            if isinstance(stmt, ast.FunctionDef):
                self.check_def(stmt)
            else:
                self.check_stmt(stmt)

    def check_def(self, fn: ast.FunctionDef) -> None:
        if fn.type_params:
            raise self.fail("type parameters", fn)
        for decorator in fn.decorator_list:
            self.check_expr(decorator)
        # Annotations are kept as source and evaluated lazily by the runtime.
        self.check_defaults(fn.args)
        self.depth += 1
        for stmt in fn.body:
            self.check_stmt(stmt)
        self.depth -= 1

    def check_defaults(self, args: ast.arguments) -> None:
        for default in args.defaults + args.kw_defaults:
            if default is not None:
                self.check_expr(default)

    def check_stmt(self, stmt: ast.stmt) -> None:
        match stmt:
            case ast.Expr(value):
                self.check_expr(value)
            case ast.Assign(targets, value):
                for target in targets:
                    self.check_target(target)
                self.check_expr(value)
            case ast.AugAssign(target, _, value):
                if not isinstance(target, ast.Name | ast.Attribute | ast.Subscript):
                    raise self.unsupported(target)
                self.check_target(target)
                self.check_expr(value)
            case ast.Delete(targets):
                for target in targets:
                    self.check_target(target, starred=False)
            case ast.Return(value):
                if value is not None:
                    self.check_expr(value)
            case ast.If(test, body, orelse):
                self.check_expr(test)
                self.check_body(body)
                self.check_body(orelse)
            case ast.While(test, body, orelse):
                self.check_expr(test)
                self.check_body(body)
                self.check_body(orelse)
            case ast.For(target, iter_, body, orelse):
                self.check_target(target)
                self.check_expr(iter_)
                self.check_body(body)
                self.check_body(orelse)
            case ast.Import():
                pass
            case ast.ImportFrom(_, names, level):
                if level:
                    raise self.fail("relative import", stmt)
                if any(alias.name == "*" for alias in names):
                    raise self.fail("star import", stmt)
            case ast.Assert(test, msg):
                self.check_expr(test)
                if msg is not None:
                    self.check_expr(msg)
            case ast.Pass() | ast.Break() | ast.Continue() | ast.Global() | ast.Nonlocal():
                pass
            case ast.AnnAssign() if self.depth == 0:
                raise self.fail("annotated assignment at module level", stmt)
            case ast.FunctionDef() if self.depth > 0:
                self.check_def(stmt)
            case ast.FunctionDef():
                raise self.fail("function definition outside module top level", stmt)
            case _:
                raise self.unsupported(stmt)

    def check_body(self, body: list[ast.stmt]) -> None:
        for stmt in body:
            self.check_stmt(stmt)

    def check_target(self, target: ast.expr, starred: bool = True) -> None:
        """Assignment, `for` and `del` targets."""
        match target:
            case ast.Name():
                pass
            case ast.Attribute(value):
                self.check_expr(value)
            case ast.Subscript(value, index):
                self.check_expr(value)
                self.check_index(index)
            case ast.Tuple(elts) | ast.List(elts):
                for elt in elts:
                    if isinstance(elt, ast.Starred):
                        if not starred:
                            raise self.unsupported(elt)
                        self.check_target(elt.value)
                    else:
                        self.check_target(elt, starred)
            case _:
                raise self.unsupported(target)

    def check_index(self, index: ast.expr) -> None:
        """The part between the brackets of a subscript, where slices may appear."""
        match index:
            case ast.Slice(lower, upper, step):
                for part in (lower, upper, step):
                    if part is not None:
                        self.check_expr(part)
            case ast.Tuple(elts):
                for elt in elts:
                    self.check_index(elt)
            case _:
                self.check_expr(index)

    def check_expr(self, expr: ast.expr) -> None:
        match expr:
            case ast.Constant(value):
                if value is not Ellipsis and not isinstance(
                    value, int | float | str | bytes | None
                ):
                    raise self.fail(f"{type(value).__name__} constant", expr)
            case ast.Name():
                pass
            case ast.UnaryOp(_, operand):
                self.check_expr(operand)
            case ast.BinOp(left, _, right):
                self.check_expr(left)
                self.check_expr(right)
            case ast.BoolOp(_, values):
                for value in values:
                    self.check_expr(value)
            case ast.Compare(left, _, comparators):
                self.check_expr(left)
                for comparator in comparators:
                    self.check_expr(comparator)
            case ast.IfExp(test, body, orelse):
                self.check_expr(test)
                self.check_expr(body)
                self.check_expr(orelse)
            case ast.Call(func, args, keywords):
                self.check_expr(func)
                for arg in args:
                    self.check_expr(arg.value if isinstance(arg, ast.Starred) else arg)
                for keyword in keywords:
                    self.check_expr(keyword.value)
            case ast.Lambda(args, body):
                self.check_defaults(args)
                self.depth += 1
                self.check_expr(body)
                self.depth -= 1
            case ast.Attribute(value):
                self.check_expr(value)
            case ast.Subscript(value, index):
                self.check_expr(value)
                self.check_index(index)
            case ast.List(elts) | ast.Tuple(elts) | ast.Set(elts):
                for elt in elts:
                    if isinstance(elt, ast.Starred):
                        raise self.fail("unpacking in a display", elt)
                    self.check_expr(elt)
            case ast.Dict(keys, values):
                for key, value in zip(keys, values, strict=True):
                    if key is None:
                        raise self.fail("unpacking in a display", value)
                    self.check_expr(key)
                    self.check_expr(value)
            case ast.JoinedStr(values):
                for value in values:
                    if isinstance(value, ast.FormattedValue):
                        self.check_expr(value.value)
                        if value.format_spec is not None:
                            self.check_expr(value.format_spec)
            case _:
                raise self.unsupported(expr)


def check_supported(module: ast.Module, filename: str) -> None:
    """Raise `UnsupportedError` for the first construct outside the supported subset."""
    SubsetChecker(filename).check_module(module)
