"""Find Python constructs that have no lowering yet.

Running this before any analysis keeps the analyses and the lowering simple:
they only ever see the supported subset. Every unsupported construct is
recorded with its source location, grouped by the top-level statement that
contains it. In mixed mode (the default) those statements are kept as Python
source (design §13); with `--no-fallback` the first one is an error.
"""

from __future__ import annotations

import ast
from dataclasses import dataclass

from steelix.diagnostics import SourceLocation, UnsupportedError
from steelix.frontend.scope import comprehension_variables

# Human readable names for the error message, by AST node type.
_FEATURES: dict[type[ast.AST], str] = {
    ast.AsyncFor: "async for",
    ast.AnnAssign: "annotated assignment",
    ast.TryStar: "try/except*",
    ast.AsyncWith: "async with",
    ast.ClassDef: "class definition",
    ast.AsyncFunctionDef: "async function",
    ast.Match: "match statement",
    ast.TypeAlias: "type alias",
    ast.Slice: "slice outside a subscript",
    ast.Starred: "starred expression",
    ast.Await: "await",
    ast.Yield: "yield",
    ast.YieldFrom: "yield from",
    ast.TemplateStr: "t-string",
}


FunctionLike = ast.FunctionDef | ast.AsyncFunctionDef

CLASS_ANNOTATION_IN_BLOCK = "annotation in a compound statement of a class body"
CLASS_GLOBAL = "global or nonlocal declaration in a class body"
WHOLE_CLASS_FEATURES = frozenset({CLASS_ANNOTATION_IN_BLOCK, CLASS_GLOBAL})
"""Unsupported constructs in a class body that keep the whole class as
Python: they change how the rest of the body is compiled."""


@dataclass(frozen=True, eq=False)
class Issue:
    """One unsupported construct."""

    feature: str
    location: SourceLocation
    owner: FunctionLike | None = None
    """The outermost function (a module level `def` or a method) containing
    the construct, `None` outside of functions."""

    def error(self) -> UnsupportedError:
        return UnsupportedError(self.feature, self.location)


class SubsetChecker:
    def __init__(self, filename: str) -> None:
        self.filename = filename
        self.depth = 0
        """Number of enclosing functions and classes."""
        self.functions: list[FunctionLike] = []
        """The enclosing functions, outermost first."""
        self.blocks: list[str] = []
        """`class` and `function` for each enclosing class body and function."""
        self.issues: list[Issue] = []

    def fail(self, feature: str, node: ast.AST) -> None:
        owner = self.functions[0] if self.functions else None
        self.issues.append(Issue(feature, SourceLocation.of(self.filename, node), owner))

    def unsupported(self, node: ast.AST) -> None:
        """Record `node` as unsupported, then look for more inside it."""
        self.fail(_FEATURES.get(type(node), type(node).__name__), node)
        self.descend(node)

    def descend(self, node: ast.AST) -> None:
        """Check the children of an unsupported node, so that every
        unsupported construct is found, not only the outermost one."""
        for child in ast.iter_child_nodes(node):
            match child:
                case ast.stmt():
                    self.check_stmt(child)
                case ast.Starred() | ast.Slice() | ast.FormattedValue() | ast.Interpolation():
                    # Parts of a larger expression, not unsupported themselves.
                    self.descend(child)
                case ast.expr():
                    self.check_expr(child)
                case _:
                    self.descend(child)

    def check_module(self, module: ast.Module) -> dict[ast.stmt, list[Issue]]:
        """The unsupported constructs of each top-level statement that has any."""
        found: dict[ast.stmt, list[Issue]] = {}
        for stmt in module.body:
            self.issues = []
            if isinstance(stmt, ast.FunctionDef):
                self.check_def(stmt)
            else:
                self.check_stmt(stmt)
            if self.issues:
                found[stmt] = self.issues
        return found

    def check_def(self, fn: FunctionLike) -> None:
        if fn.type_params:
            self.fail("type parameters", fn)
        for decorator in fn.decorator_list:
            self.check_expr(decorator)
        # Annotations are kept as source and evaluated lazily by the runtime.
        self.check_defaults(fn.args)
        self.depth += 1
        self.functions.append(fn)
        self.blocks.append("function")
        for stmt in fn.body:
            self.check_stmt(stmt)
        self.blocks.pop()
        self.functions.pop()
        self.depth -= 1

    def in_class_body(self) -> bool:
        return bool(self.blocks) and self.blocks[-1] == "class"

    def check_class(self, cls: ast.ClassDef) -> None:
        if cls.type_params:
            self.fail("type parameters", cls)
        for decorator in cls.decorator_list:
            self.check_expr(decorator)
        for base in cls.bases:
            self.check_expr(base.value if isinstance(base, ast.Starred) else base)
        for keyword in cls.keywords:
            self.check_expr(keyword.value)
        self.depth += 1
        self.blocks.append("class")
        for stmt in cls.body:
            match stmt:
                case ast.AnnAssign(ast.Name(), _, value, simple=1):
                    # Evaluated lazily by the class's `__annotate__`.
                    if value is not None:
                        self.check_expr(value)
                case ast.AnnAssign():
                    self.fail("annotated assignment to an attribute or subscript", stmt)
                    self.descend(stmt)
                case _:
                    self.check_stmt(stmt)
        self.blocks.pop()
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
                    self.unsupported(target)
                else:
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
            case ast.ImportFrom(_, names):
                if any(alias.name == "*" for alias in names):
                    self.fail("star import", stmt)
            case ast.Assert(test, msg):
                self.check_expr(test)
                if msg is not None:
                    self.check_expr(msg)
            case ast.Global() | ast.Nonlocal() if self.in_class_body():
                self.fail(CLASS_GLOBAL, stmt)
            case ast.Pass() | ast.Break() | ast.Continue() | ast.Global() | ast.Nonlocal():
                pass
            case ast.Raise(exc, cause):
                if exc is not None:
                    self.check_expr(exc)
                if cause is not None:
                    self.check_expr(cause)
            case ast.Try(body, handlers, orelse, finalbody):
                self.check_body(body)
                for handler in handlers:
                    if handler.type is not None:
                        self.check_expr(handler.type)
                    self.check_body(handler.body)
                self.check_body(orelse)
                self.check_body(finalbody)
            case ast.With(items, body):
                for item in items:
                    self.check_expr(item.context_expr)
                    if item.optional_vars is not None:
                        self.check_target(item.optional_vars)
                self.check_body(body)
            case ast.AnnAssign() if self.in_class_body():
                self.fail(CLASS_ANNOTATION_IN_BLOCK, stmt)
                self.descend(stmt)
            case ast.AnnAssign() if self.depth == 0:
                self.fail("annotated assignment at module level", stmt)
                self.descend(stmt)
            case ast.FunctionDef():
                self.check_def(stmt)
            case ast.AsyncFunctionDef():
                self.fail("async function", stmt)
                self.check_def(stmt)
            case ast.ClassDef() if self.functions:
                # The methods are checked too: what else blocks them is worth
                # knowing (coverage report).
                self.fail("class definition in a function", stmt)
                self.depth += 1
                self.descend(stmt)
                self.depth -= 1
            case ast.ClassDef():
                self.check_class(stmt)
            case _:
                self.unsupported(stmt)

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
                            self.unsupported(elt)
                        else:
                            self.check_target(elt.value)
                    else:
                        self.check_target(elt, starred)
            case _:
                self.unsupported(target)

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
                    self.fail(f"{type(value).__name__} constant", expr)
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
                    self.check_expr(elt.value if isinstance(elt, ast.Starred) else elt)
            case ast.Dict(keys, values):
                for key, value in zip(keys, values, strict=True):
                    if key is not None:
                        self.check_expr(key)
                    self.check_expr(value)
            case ast.JoinedStr(values):
                for value in values:
                    if isinstance(value, ast.FormattedValue):
                        self.check_expr(value.value)
                        if value.format_spec is not None:
                            self.check_expr(value.format_spec)
            case ast.NamedExpr(_, value):
                self.check_expr(value)
            case ast.ListComp() | ast.SetComp() | ast.DictComp() | ast.GeneratorExp():
                self.check_comprehension(expr)
            case _:
                self.unsupported(expr)

    def check_comprehension(
        self, node: ast.ListComp | ast.SetComp | ast.DictComp | ast.GeneratorExp
    ) -> None:
        """Comprehensions and generator expressions (design §8). A generator
        expression is lowered as a single `for` loop resumed by each
        `next()`, so it has one `for` clause, and nothing may capture its
        variables (they would have to be cells living as long as the
        generator)."""
        generator_exp = isinstance(node, ast.GeneratorExp)
        if any(g.is_async for g in node.generators):
            self.fail("asynchronous comprehension", node)
        if generator_exp and len(node.generators) > 1:
            self.fail("generator expression with several for clauses", node)
        variables = comprehension_variables(node)
        if generator_exp and _captured(node, variables):
            self.fail("generator expression variable captured by a nested function", node)
        if _used_before_bound(node):
            self.fail("comprehension variable used before it is bound", node)
        for generator in node.generators:
            self.check_target(generator.target)
            self.check_expr(generator.iter)
            for test in generator.ifs:
                self.check_expr(test)
        if isinstance(node, ast.DictComp):
            self.check_expr(node.key)
            self.check_expr(node.value)
        else:
            self.check_expr(node.elt)


def _captured(node: ast.GeneratorExp, variables: frozenset[str]) -> bool:
    """Whether a function, lambda or generator expression nested in `node`
    refers to one of its `variables` (in its body: defaults and the first
    iterable of a generator expression are evaluated in `node`)."""
    for child in ast.walk(node):
        match child:
            case ast.FunctionDef(body=body) | ast.Lambda(body=body):
                inner = body if isinstance(body, list) else [body]
            case ast.GeneratorExp(generators=generators) if child is not node:
                first, *rest = generators
                inner = [first.target, *first.ifs, *rest, child.elt]
            case _:
                continue
        for part in inner:
            for name in ast.walk(part):
                if isinstance(name, ast.Name) and name.id in variables:
                    return True
    return False


def _used_before_bound(node: ast.ListComp | ast.SetComp | ast.DictComp | ast.GeneratorExp) -> bool:
    """Whether a variable of the comprehension is read before the `for`
    clause binding it (`[y for x in xs for y in f(y)]`), which raises
    `UnboundLocalError` in CPython. The lowering binds each variable at its
    `for` clause and would read an outer variable instead."""
    variables = comprehension_variables(node)
    bound: set[str] = set()
    for index, generator in enumerate(node.generators):
        if index:
            if _reads(generator.iter, variables - bound):
                return True
        bound |= {n.id for n in ast.walk(generator.target) if isinstance(n, ast.Name)}
        if any(_reads(test, variables - bound) for test in generator.ifs):
            return True
    return False


def _reads(node: ast.expr, names: set[str] | frozenset[str]) -> bool:
    return any(isinstance(n, ast.Name) and n.id in names for n in ast.walk(node))


def find_unsupported(module: ast.Module, filename: str) -> dict[ast.stmt, list[Issue]]:
    """The unsupported constructs of `module`, by top-level statement, in
    source order."""
    return SubsetChecker(filename).check_module(module)


def check_supported(module: ast.Module, filename: str) -> None:
    """Raise `UnsupportedError` for the first construct outside the supported subset."""
    for issues in find_unsupported(module, filename).values():
        raise issues[0].error()
