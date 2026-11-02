"""Scope analysis based on the standard library `symtable` module.

`symtable` runs CPython's own scope analysis, so local / global / free
classification matches the interpreter exactly. On top of it this module links
every function scope (`def` or `lambda`, at any depth) to its AST node, and
works out which locals are cells, i.e. captured by a nested function.
"""

from __future__ import annotations

import ast
import symtable
from collections.abc import Collection, Iterator
from dataclasses import dataclass, field

from steelix.diagnostics import SourceLocation, UnsupportedError

FunctionNode = ast.FunctionDef | ast.Lambda


@dataclass(eq=False)
class FunctionScope:
    name: str
    params: tuple[str, ...]
    """Parameters in source order: positional-only, positional, `*args`,
    keyword-only, `**kwargs`."""
    locals: frozenset[str]
    """Local names including parameters and cells."""
    globals: frozenset[str]
    """Names this function uses from module globals or builtins."""
    frees: tuple[str, ...] = ()
    """Variables captured from enclosing functions, in a stable order."""
    nonlocals: frozenset[str] = frozenset()
    cells: set[str] = field(default_factory=set)
    """Locals that nested functions capture."""
    node: FunctionNode | None = None
    parent: FunctionScope | None = None
    """The enclosing function, `None` at module level."""
    children: list[FunctionScope] = field(default_factory=list)
    qualname: str = ""

    def is_local(self, name: str) -> bool:
        return name in self.locals

    def is_free(self, name: str) -> bool:
        return name in self.frees

    def is_global(self, name: str) -> bool:
        return name not in self.locals and name not in self.frees

    def owner(self, name: str) -> FunctionScope:
        """The function whose local a free variable `name` is."""
        scope = self.parent
        while scope is not None:
            if scope.is_local(name):
                return scope
            scope = scope.parent
        raise AssertionError(f"free variable {name!r} has no owner")

    def descendants(self) -> Iterator[FunctionScope]:
        for child in self.children:
            yield child
            yield from child.descendants()


@dataclass
class ModuleScopes:
    functions: dict[FunctionNode, FunctionScope]
    """Every function scope, nested ones included."""
    top_level: list[FunctionScope]

    def of(self, fn: FunctionNode) -> FunctionScope:
        return self.functions[fn]


def walk_scope(node: ast.AST, skip: Collection[ast.AST] = ()) -> Iterator[ast.AST]:
    """Like `ast.walk`, but only the nodes evaluated in the scope of `node`.

    Nested `def` and `lambda` nodes are yielded themselves, together with the
    parts evaluated where they are defined (defaults and decorators), but not
    their bodies. Annotations are evaluated lazily in their own scope (PEP
    649) and are skipped too. The order follows `symtable`, which visits
    defaults before decorators and both before the function itself. Nodes in
    `skip` (statements kept as Python) are left out with everything in them.
    """
    if node in skip:
        return
    match node:
        case ast.FunctionDef(args=args, decorator_list=decorators):
            yield from _walk_all([*args.defaults, *_present(args.kw_defaults), *decorators])
            yield node
        case ast.Lambda(args=args):
            yield from _walk_all([*args.defaults, *_present(args.kw_defaults)])
            yield node
        case _:
            yield node
            for child in ast.iter_child_nodes(node):
                yield from walk_scope(child, skip)


def _walk_all(nodes: list[ast.expr]) -> Iterator[ast.AST]:
    for node in nodes:
        yield from walk_scope(node)


def _present(nodes: list[ast.expr | None]) -> list[ast.expr]:
    return [n for n in nodes if n is not None]


def scope_body(fn: FunctionNode) -> list[ast.AST]:
    """The nodes making up a function's own scope."""
    return list(fn.body) if isinstance(fn, ast.FunctionDef) else [fn.body]


def nested_functions(nodes: list[ast.AST], skip: Collection[ast.AST] = ()) -> list[FunctionNode]:
    return [n for node in nodes for n in walk_scope(node, skip) if isinstance(n, FunctionNode)]


def params_of(args: ast.arguments) -> tuple[str, ...]:
    names = [a.arg for a in args.posonlyargs + args.args]
    if args.vararg is not None:
        names.append(args.vararg.arg)
    names += [a.arg for a in args.kwonlyargs]
    if args.kwarg is not None:
        names.append(args.kwarg.arg)
    return tuple(names)


def default_of(args: ast.arguments, param: str) -> ast.expr:
    """The default value expression of the parameter `param`."""
    positional = args.posonlyargs + args.args
    first_default = len(positional) - len(args.defaults)
    for i, arg in enumerate(positional):
        if arg.arg == param and i >= first_default:
            return args.defaults[i - first_default]
    for arg, default in zip(args.kwonlyargs, args.kw_defaults, strict=True):
        if arg.arg == param and default is not None:
            return default
    raise KeyError(param)


def analyze_scopes(
    module: ast.Module,
    source: str,
    filename: str,
    python: Collection[ast.stmt] = (),
) -> ModuleScopes:
    """Scopes of the functions in `module`, except in the statements
    `python` that are kept as Python source."""
    table = symtable.symtable(source, filename, "exec")
    functions: dict[FunctionNode, FunctionScope] = {}
    top_level = _link(table, list(module.body), None, filename, functions, python)
    for scope in functions.values():
        for name in scope.frees:
            scope.owner(name).cells.add(name)
    future_annotations = has_future_annotations(module)
    for scope in functions.values():
        if not future_annotations:
            _check_annotations(scope, filename)
    return ModuleScopes(functions, top_level)


def _link(
    table: symtable.SymbolTable,
    body: list[ast.AST],
    parent: FunctionScope | None,
    filename: str,
    functions: dict[FunctionNode, FunctionScope],
    skip: Collection[ast.AST] = (),
) -> list[FunctionScope]:
    """Match the function tables nested in `table` with the AST nodes,
    except those in `skip`."""
    tables: dict[tuple[str, int], list[symtable.Function]] = {}
    for child in table.get_children():
        if child.get_type() == symtable.SymbolTableType.FUNCTION:
            assert isinstance(child, symtable.Function)
            tables.setdefault((child.get_name(), child.get_lineno()), []).append(child)
    scopes = []
    for node in nested_functions(body, skip):
        name = node.name if isinstance(node, ast.FunctionDef) else "lambda"
        child = tables[(name, node.lineno)].pop(0)
        scope = _function_scope(child, node, parent)
        functions[node] = scope
        scope.children = _link(child, scope_body(node), scope, filename, functions)
        scopes.append(scope)
    return scopes


def _function_scope(
    table: symtable.Function, node: FunctionNode, parent: FunctionScope | None
) -> FunctionScope:
    globals_ = set()
    for symbol in table.get_symbols():
        # Includes names declared with a `global` statement.
        if symbol.is_global():
            globals_.add(symbol.get_name())
    name = node.name if isinstance(node, ast.FunctionDef) else "<lambda>"
    qualname = name if parent is None else f"{parent.qualname}.<locals>.{name}"
    return FunctionScope(
        name=name,
        params=params_of(node.args),
        locals=frozenset(table.get_locals()),
        globals=frozenset(globals_),
        frees=tuple(table.get_frees()),
        nonlocals=frozenset(table.get_nonlocals()),
        node=node,
        parent=parent,
        qualname=qualname,
    )


def has_future_annotations(module: ast.Module) -> bool:
    return any(
        isinstance(stmt, ast.ImportFrom)
        and stmt.module == "__future__"
        and any(alias.name == "annotations" for alias in stmt.names)
        for stmt in module.body
    )


def annotations_of(fn: ast.FunctionDef) -> list[tuple[str, ast.expr]]:
    """`(parameter, annotation)` pairs in the order of `__annotations__`."""
    args = fn.args
    params = [*args.posonlyargs, *args.args]
    if args.vararg is not None:
        params.append(args.vararg)
    params += args.kwonlyargs
    if args.kwarg is not None:
        params.append(args.kwarg)
    pairs = [(a.arg, a.annotation) for a in params if a.annotation is not None]
    if fn.returns is not None:
        pairs.append(("return", fn.returns))
    return pairs


def _check_annotations(scope: FunctionScope, filename: str) -> None:
    """Annotations are evaluated in the module globals (design §6), so they
    must not refer to variables of an enclosing function."""
    fn = scope.node
    if not isinstance(fn, ast.FunctionDef) or scope.parent is None:
        return
    for _, annotation in annotations_of(fn):
        for node in ast.walk(annotation):
            if isinstance(node, ast.Name) and _resolves_to_function(scope.parent, node.id):
                raise UnsupportedError(
                    "annotation using a variable of an enclosing function",
                    SourceLocation.of(filename, node),
                )


def _resolves_to_function(scope: FunctionScope | None, name: str) -> bool:
    while scope is not None:
        if scope.is_local(name) or scope.is_free(name):
            return True
        scope = scope.parent
    return False


MODULE_SCOPE = FunctionScope(name="<module>", params=(), locals=frozenset(), globals=frozenset())
"""Module level code has no locals: every name is a global."""
