"""Rewrite constructs that have a simpler equivalent before the analyses run."""

from __future__ import annotations

import ast


class _LocalAnnotations(ast.NodeTransformer):
    """Annotated assignments in function bodies.

    The annotation of a local variable is never evaluated (PEP 526, PEP 649),
    so `x: int = v` is just `x = v`, and `x: int` only makes `x` local, which
    `symtable` already accounts for.
    """

    def __init__(self) -> None:
        self.depth = 0

    def visit_FunctionDef(self, node: ast.FunctionDef) -> ast.AST:
        self.depth += 1
        self.generic_visit(node)
        self.depth -= 1
        return node

    visit_AsyncFunctionDef = visit_FunctionDef

    def visit_ClassDef(self, node: ast.ClassDef) -> ast.AST:
        # Annotations in a class body are the class's, never locals.
        saved, self.depth = self.depth, 0
        self.generic_visit(node)
        self.depth = saved
        return node

    def visit_AnnAssign(self, node: ast.AnnAssign) -> ast.AST:
        if self.depth == 0:
            return node
        if node.value is not None:
            return ast.copy_location(ast.Assign([node.target], node.value), node)
        if isinstance(node.target, ast.Name):
            return ast.copy_location(ast.Pass(), node)
        # `obj.attr: int` still evaluates `obj`; left to the subset check.
        return node


def mangle(private: str, name: str) -> str:
    """CPython's private name mangling (`_Py_Mangle`): `__x` in the class
    `Point` is `_Point__x`. `private` is the name of the innermost class."""
    if not name.startswith("__") or name.endswith("__") or "." in name:
        return name
    stripped = private.lstrip("_")
    if not stripped:
        return name
    return f"_{stripped}{name}"


class _Mangle(ast.NodeVisitor):
    """Private names in classes, mangled like the compiler does, so that the
    AST agrees with `symtable` (which reports mangled names) and the lowering
    spells the names CPython uses at run time.

    The names of `def` and `class` statements are left alone: `__name__`
    keeps the plain name, only the binding is mangled (`binding_name`).
    Keyword arguments of calls are not mangled, by CPython either.
    """

    def __init__(self) -> None:
        self.classes: list[str] = []

    def visit_ClassDef(self, node: ast.ClassDef) -> None:
        self.bind(node)
        for child in [*node.decorator_list, *node.bases, *node.keywords]:
            self.visit(child)
        self.classes.append(node.name)
        for stmt in node.body:
            self.visit(stmt)
        self.classes.pop()

    def visit_FunctionDef(self, node: ast.FunctionDef) -> None:
        self.bind(node)
        self.keep_source(node.returns)
        self.generic_visit(node)

    def visit_AsyncFunctionDef(self, node: ast.AsyncFunctionDef) -> None:
        self.bind(node)
        self.keep_source(node.returns)
        self.generic_visit(node)

    def visit_AnnAssign(self, node: ast.AnnAssign) -> None:
        self.keep_source(node.annotation)
        self.generic_visit(node)

    def keep_source(self, annotation: ast.expr | None) -> None:
        """Remember the text of `annotation` before mangling: with `from
        __future__ import annotations` it is the string CPython stores."""
        if annotation is not None and self.classes:
            annotation._steelix_source = ast.unparse(annotation)  # type: ignore[attr-defined]

    def bind(self, node: ast.FunctionDef | ast.AsyncFunctionDef | ast.ClassDef) -> None:
        if self.fix(node.name) != node.name:
            node._steelix_binding = self.fix(node.name)  # type: ignore[union-attr]

    def fix(self, name: str) -> str:
        return mangle(self.classes[-1], name) if self.classes else name

    def visit_Name(self, node: ast.Name) -> None:
        node.id = self.fix(node.id)

    def visit_Attribute(self, node: ast.Attribute) -> None:
        node.attr = self.fix(node.attr)
        self.visit(node.value)

    def visit_arg(self, node: ast.arg) -> None:
        node.arg = self.fix(node.arg)
        self.keep_source(node.annotation)
        if node.annotation is not None:
            self.visit(node.annotation)

    def visit_ExceptHandler(self, node: ast.ExceptHandler) -> None:
        if node.name is not None:
            node.name = self.fix(node.name)
        self.generic_visit(node)

    def visit_Global(self, node: ast.Global) -> None:
        node.names = [self.fix(n) for n in node.names]

    def visit_Nonlocal(self, node: ast.Nonlocal) -> None:
        node.names = [self.fix(n) for n in node.names]

    def visit_alias(self, node: ast.alias) -> None:
        bound = node.asname or node.name
        if node.name != "*" and "." not in bound and self.fix(bound) != bound:
            node.asname = self.fix(bound)


def binding_name(node: ast.FunctionDef | ast.AsyncFunctionDef | ast.ClassDef) -> str:
    """The name a `def` or `class` statement binds: its name, mangled in a
    class (`_Mangle`)."""
    return getattr(node, "_steelix_binding", node.name)


def annotation_source(annotation: ast.expr) -> str:
    """The source text of an annotation as `from __future__ import
    annotations` stores it: unparsed, with private names not mangled."""
    return getattr(annotation, "_steelix_source", None) or ast.unparse(annotation)


class AsyncFunctionDef(ast.FunctionDef):
    """`async def`, as a kind of `def` (`desugar`)."""


class AsyncFor(ast.For):
    """`async for`, as a kind of `for` (`desugar`)."""


class AsyncWith(ast.With):
    """`async with`, as a kind of `with` (`desugar`)."""


_ASYNC = {
    ast.AsyncFunctionDef: AsyncFunctionDef,
    ast.AsyncFor: AsyncFor,
    ast.AsyncWith: AsyncWith,
}


def _async_as_subclasses(module: ast.Module) -> None:
    """Replace `async def`, `async for` and `async with` nodes by the
    subclasses of `def`, `for` and `with` above, which have the same fields
    and class names: the analyses treat them like their plain forms (scopes,
    bindings, regions), the lowering tells them apart with `isinstance`, and
    `ast.unparse` and visitors still see `async` nodes."""
    for node in ast.walk(module):
        if isinstance(node, ast.Await | ast.YieldFrom) and isinstance(node.value, ast.Call):
            node.value._steelix_suspends = (  # type: ignore[attr-defined]
                "await" if isinstance(node, ast.Await) else "yield from"
            )
        for name, value in ast.iter_fields(node):
            if isinstance(value, list):
                for i, item in enumerate(value):
                    if type(item) in _ASYNC:
                        value[i] = _converted(item)
            elif type(value) in _ASYNC:
                setattr(node, name, _converted(value))


def _converted(node: ast.AST) -> ast.AST:
    new = _ASYNC[type(node)](**dict(ast.iter_fields(node)))
    for attr in node._attributes:
        if hasattr(node, attr):
            setattr(new, attr, getattr(node, attr))
    new.__dict__.update({k: v for k, v in node.__dict__.items() if k.startswith("_steelix")})
    return new


def suspends(call: ast.Call) -> str | None:
    """`await` for the call in `await f(x)`, `yield from` for the one in
    `yield from g(x)`: the only calls that can go straight to the Rust fn
    of a native coroutine or generator (design §8)."""
    return getattr(call, "_steelix_suspends", None)


def desugar(module: ast.Module) -> None:
    """Rewrite `module` in place."""
    _Mangle().visit(module)
    _LocalAnnotations().visit(module)
    _async_as_subclasses(module)
