"""Reject Python constructs that have no lowering yet.

Running this before any analysis keeps the analyses and the lowering simple:
they only ever see the supported subset, and every unsupported construct is
reported with its source location instead of producing wrong code.
"""

from __future__ import annotations

import ast

from steelix.diagnostics import SourceLocation, UnsupportedError

_COMPARE_OPS = (ast.Lt, ast.LtE, ast.Gt, ast.GtE, ast.Eq, ast.NotEq)

# Human readable names for the error message, by AST node type.
_FEATURES: dict[type[ast.AST], str] = {
    ast.For: "for loop",
    ast.AsyncFor: "async for",
    ast.AugAssign: "augmented assignment",
    ast.AnnAssign: "annotated assignment",
    ast.Break: "break",
    ast.Continue: "continue",
    ast.Import: "import",
    ast.ImportFrom: "import",
    ast.Global: "global statement",
    ast.Nonlocal: "nonlocal statement",
    ast.Try: "try statement",
    ast.TryStar: "try/except*",
    ast.Raise: "raise",
    ast.With: "with statement",
    ast.AsyncWith: "async with",
    ast.ClassDef: "class definition",
    ast.AsyncFunctionDef: "async function",
    ast.Delete: "del statement",
    ast.Assert: "assert",
    ast.Match: "match statement",
    ast.TypeAlias: "type alias",
    ast.BoolOp: "and/or",
    ast.UnaryOp: "unary operator",
    ast.IfExp: "conditional expression",
    ast.Lambda: "lambda",
    ast.NamedExpr: "assignment expression",
    ast.Attribute: "attribute access",
    ast.Subscript: "subscript",
    ast.Slice: "slice",
    ast.Starred: "starred expression",
    ast.List: "list display",
    ast.Tuple: "tuple display",
    ast.Dict: "dict display",
    ast.Set: "set display",
    ast.ListComp: "list comprehension",
    ast.SetComp: "set comprehension",
    ast.DictComp: "dict comprehension",
    ast.GeneratorExp: "generator expression",
    ast.Await: "await",
    ast.Yield: "yield",
    ast.YieldFrom: "yield from",
    ast.JoinedStr: "f-string",
    ast.TemplateStr: "t-string",
}


class SubsetChecker:
    def __init__(self, filename: str) -> None:
        self.filename = filename

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
        if fn.decorator_list:
            raise self.fail("decorator", fn.decorator_list[0])
        if fn.type_params:
            raise self.fail("type parameters", fn)
        if fn.returns is not None:
            raise self.fail("annotation", fn.returns)
        args = fn.args
        if args.vararg or args.kwarg:
            raise self.fail("*args / **kwargs parameter", fn)
        if args.kwonlyargs:
            raise self.fail("keyword-only parameter", fn)
        if args.defaults:
            raise self.fail("default argument", args.defaults[0])
        for arg in args.posonlyargs + args.args:
            if arg.annotation is not None:
                raise self.fail("annotation", arg.annotation)
        for stmt in fn.body:
            if isinstance(stmt, ast.FunctionDef):
                raise self.fail("nested function", stmt)
            self.check_stmt(stmt)

    def check_stmt(self, stmt: ast.stmt) -> None:
        match stmt:
            case ast.Expr(value):
                self.check_expr(value)
            case ast.Assign(targets, value):
                for target in targets:
                    if not isinstance(target, ast.Name):
                        raise self.unsupported(target)
                self.check_expr(value)
            case ast.Return(value):
                if value is not None:
                    self.check_expr(value)
            case ast.If(test, body, orelse):
                self.check_expr(test)
                self.check_body(body)
                self.check_body(orelse)
            case ast.While(test, body, orelse):
                if orelse:
                    raise self.fail("while/else", orelse[0])
                self.check_expr(test)
                self.check_body(body)
            case ast.Pass():
                pass
            case ast.FunctionDef():
                raise self.fail("function definition outside module top level", stmt)
            case _:
                raise self.unsupported(stmt)

    def check_body(self, body: list[ast.stmt]) -> None:
        for stmt in body:
            self.check_stmt(stmt)

    def check_expr(self, expr: ast.expr) -> None:
        match expr:
            case ast.Constant(value):
                if not isinstance(value, int | float | str | bytes | None):
                    raise self.fail(f"{type(value).__name__} constant", expr)
            case ast.UnaryOp(ast.USub(), ast.Constant(value)) if _is_number(value):
                pass  # a negative literal
            case ast.Name():
                pass
            case ast.BinOp(left, _, right):
                self.check_expr(left)
                self.check_expr(right)
            case ast.Compare(left, ops, comparators):
                if len(ops) > 1:
                    raise self.fail("chained comparison", expr)
                if not isinstance(ops[0], _COMPARE_OPS):
                    raise self.fail(f"'{_compare_text(ops[0])}' comparison", expr)
                self.check_expr(left)
                self.check_expr(comparators[0])
            case ast.Call(func, args, keywords):
                if not isinstance(func, ast.Name):
                    raise self.fail("call of a non-name expression", func)
                if keywords:
                    raise self.fail("keyword argument", keywords[0])
                for arg in args:
                    if isinstance(arg, ast.Starred):
                        raise self.fail("argument unpacking", arg)
                    self.check_expr(arg)
            case _:
                raise self.unsupported(expr)


def _is_number(value: object) -> bool:
    return isinstance(value, int | float) and not isinstance(value, bool)


def _compare_text(op: ast.cmpop) -> str:
    return {ast.In: "in", ast.NotIn: "not in", ast.Is: "is", ast.IsNot: "is not"}.get(
        type(op), type(op).__name__
    )


def check_supported(module: ast.Module, filename: str) -> None:
    """Raise `UnsupportedError` for the first construct outside the M1 subset."""
    SubsetChecker(filename).check_module(module)
