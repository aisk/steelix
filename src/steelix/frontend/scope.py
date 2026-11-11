"""Scope analysis based on the standard library `symtable` module.

`symtable` runs CPython's own scope analysis, so local / global / free
classification matches the interpreter exactly. On top of it this module links
every function scope (`def`, `lambda` or generator expression, at any depth)
to its AST node, and works out which locals are cells, i.e. captured by a
nested function.

List, set and dict comprehensions are inlined into the enclosing scope since
Python 3.12 (PEP 709), and `symtable` reports their iteration variables as
locals of the enclosing function. They are still separate variables, which
the lowering declares inside the comprehension's loop (design §8). The
analyses here look at function code through `walk_scope`, which leaves out
every use of a comprehension's variables inside the comprehension.
"""

from __future__ import annotations

import ast
import symtable
from collections.abc import Collection, Iterator
from dataclasses import dataclass, field

from steelix.diagnostics import SourceLocation, UnsupportedError
from steelix.frontend.desugar import AsyncFunctionDef

FunctionNode = ast.FunctionDef | ast.Lambda | ast.GeneratorExp
Comprehension = ast.ListComp | ast.SetComp | ast.DictComp


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
    comprehension_frees: frozenset[str] = frozenset()
    """Free variables that are variables of a comprehension the function is
    defined in (`[lambda: x for x in xs]`): the comprehension's cell, not one
    of the enclosing function."""

    def is_local(self, name: str) -> bool:
        return name in self.locals

    def is_free(self, name: str) -> bool:
        return name in self.frees

    def is_global(self, name: str) -> bool:
        return name not in self.locals and name not in self.frees

    def owner(self, name: str) -> FunctionScope:
        """The function whose local a free variable `name` is, or
        `CLASS_SCOPE` for the `__class__` cell of a class body."""
        scope = self.parent
        while scope is not None:
            if scope.is_local(name) or name in scope.comprehension_frees:
                # A comprehension variable captured by a lambda defined in a
                # comprehension: `scope` receives its cell.
                return scope
            scope = scope.parent
        if name == "__class__":
            return CLASS_SCOPE
        raise AssertionError(f"free variable {name!r} has no owner")

    def descendants(self) -> Iterator[FunctionScope]:
        for child in self.children:
            yield child
            yield from child.descendants()


@dataclass(eq=False)
class ClassScope:
    """The body of a `class` statement."""

    name: str
    qualname: str
    node: ast.ClassDef
    parent: ClassScope | None
    """The enclosing class, if the class is nested in one."""
    globals: frozenset[str] = frozenset()
    """Names declared `global` in the class body."""
    methods: list[FunctionScope] = field(default_factory=list)
    """Functions defined directly in the body (`def` and `lambda`)."""


@dataclass
class ModuleScopes:
    functions: dict[FunctionNode, FunctionScope]
    """Every function scope, nested ones included."""
    top_level: list[FunctionScope]
    classes: dict[ast.ClassDef, ClassScope] = field(default_factory=dict)
    """Every class lowered natively (not kept as Python)."""

    def of(self, fn: FunctionNode) -> FunctionScope:
        return self.functions[fn]

    def class_of(self, node: ast.ClassDef) -> ClassScope:
        return self.classes[node]


def walk_scope(
    node: ast.AST, skip: Collection[ast.AST] = (), hidden: frozenset[str] = frozenset()
) -> Iterator[ast.AST]:
    """Like `ast.walk`, but only the nodes evaluated in the scope of `node`.

    Nested `def`, `lambda`, generator expression and `class` nodes are
    yielded themselves, together with the parts evaluated where they are
    defined (defaults, decorators, bases and the iterable of a generator
    expression's first `for`), but not their bodies. Annotations are
    evaluated lazily in their own scope (PEP 649) and are skipped too. The
    order follows `symtable`, which visits defaults before decorators and
    both before the function itself. Nodes in `skip` (statements kept as
    Python) are left out with everything in them.

    A list, set or dict comprehension is yielded with its parts, except the
    `Name` nodes of its own variables (the `for` targets and their uses
    inside it), which are not variables of this scope. `hidden` are the
    variables of the comprehensions around `node`.
    """
    if node in skip:
        return
    match node:
        case ast.ClassDef(bases=bases, keywords=keywords, decorator_list=decorators):
            # The body is a scope of its own, evaluated when the class is created.
            yield from _walk_all([*bases, *(k.value for k in keywords), *decorators], hidden)
            yield node
        case ast.FunctionDef(args=args, decorator_list=decorators):
            parts = [*args.defaults, *_present(args.kw_defaults), *decorators]
            yield from _walk_all(parts, hidden)
            yield node
        case ast.Lambda(args=args):
            yield from _walk_all([*args.defaults, *_present(args.kw_defaults)], hidden)
            yield node
        case ast.GeneratorExp(generators=generators):
            yield from walk_scope(generators[0].iter, skip, hidden)
            yield node
        case ast.ListComp() | ast.SetComp() | ast.DictComp():
            yield node
            inner = hidden | comprehension_variables(node)
            yield from walk_scope(node.generators[0].iter, skip, hidden)
            for part in comprehension_parts(node):
                yield from walk_scope(part, skip, inner)
        case ast.Name(id) if id in hidden:
            return
        case _:
            yield node
            for child in ast.iter_child_nodes(node):
                yield from walk_scope(child, skip, hidden)


def comprehension_variables(node: Comprehension | ast.GeneratorExp) -> frozenset[str]:
    """The names bound by the `for` targets of a comprehension."""
    return frozenset(
        name.id
        for generator in node.generators
        for name in ast.walk(generator.target)
        # `for xs[0] in ..` reads `xs`, a variable of the enclosing scope.
        if isinstance(name, ast.Name) and isinstance(name.ctx, ast.Store)
    )


def comprehension_parts(node: Comprehension | ast.GeneratorExp) -> list[ast.AST]:
    """The parts of a comprehension evaluated in its own scope, in the order
    `symtable` visits them: everything but the iterable of the first `for`."""
    first, *rest = node.generators
    parts: list[ast.AST] = [first.target, *first.ifs]
    for generator in rest:
        parts += [generator.target, generator.iter, *generator.ifs]
    if isinstance(node, ast.DictComp):
        parts += [node.value, node.key]
    else:
        parts.append(node.elt)
    return parts


def walrus_targets(node: ast.AST) -> list[str]:
    """Names bound by `:=` in `node` outside nested functions and classes,
    in comprehensions and generator expressions too (PEP 572: they bind in
    the enclosing scope)."""
    names = []
    for child in ast.iter_child_nodes(node):
        if isinstance(child, ast.Lambda | ast.FunctionDef | ast.AsyncFunctionDef | ast.ClassDef):
            continue
        if isinstance(child, ast.NamedExpr) and isinstance(child.target, ast.Name):
            names.append(child.target.id)
        names += walrus_targets(child)
    return names


def _walk_all(nodes: list[ast.expr], hidden: frozenset[str]) -> Iterator[ast.AST]:
    for node in nodes:
        yield from walk_scope(node, hidden=hidden)


def nested_classes(nodes: list[ast.AST], skip: Collection[ast.AST] = ()) -> list[ast.ClassDef]:
    return [n for node in nodes for n in walk_scope(node, skip) if isinstance(n, ast.ClassDef)]


def _present(nodes: list[ast.expr | None]) -> list[ast.expr]:
    return [n for n in nodes if n is not None]


def scope_body(fn: FunctionNode) -> list[ast.AST]:
    """The nodes making up a function's own scope."""
    match fn:
        case ast.FunctionDef():
            return list(fn.body)
        case ast.Lambda():
            return [fn.body]
    return comprehension_parts(fn)


def function_kind(fn: FunctionNode) -> str:
    """`function`, `generator`, `coroutine` or `async generator`: what
    calling `fn` returns, from the `yield` in its own scope and `async`."""
    if not isinstance(fn, ast.FunctionDef):
        return "generator" if isinstance(fn, ast.GeneratorExp) else "function"
    yields = any(
        isinstance(node, ast.Yield | ast.YieldFrom) for stmt in fn.body for node in walk_scope(stmt)
    )
    if isinstance(fn, AsyncFunctionDef):
        return "async generator" if yields else "coroutine"
    return "generator" if yields else "function"


def returns_unit(fn: FunctionNode) -> bool:
    """Whether `fn` only ever returns `None` without saying which value:
    a plain `def` (no coroutine or generator) whose `return` statements
    are all bare or `return None`. Its Rust fn returns `PyResult<()>`
    (design §6)."""
    if not isinstance(fn, ast.FunctionDef) or function_kind(fn) != "function":
        return False
    pending: list[ast.AST] = list(fn.body)
    while pending:
        node = pending.pop()
        match node:
            case ast.Return(ast.expr() as value):
                if not (isinstance(value, ast.Constant) and value.value is None):
                    return False
            case ast.FunctionDef() | ast.AsyncFunctionDef() | ast.ClassDef():
                continue
        if isinstance(node, ast.stmt | ast.ExceptHandler | ast.match_case):
            pending.extend(ast.iter_child_nodes(node))
    return True


def nested_functions(nodes: list[ast.AST], skip: Collection[ast.AST] = ()) -> list[FunctionNode]:
    return [n for node in nodes for n in walk_scope(node, skip) if isinstance(n, FunctionNode)]


def params_of(args: ast.arguments | None) -> tuple[str, ...]:
    if args is None:
        return ()  # a generator expression
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
    classes: dict[ast.ClassDef, ClassScope] = {}
    linker = _Linker(functions, classes, python)
    top_level = linker.link(table, list(module.body), None, None)
    for node, names in comprehension_captures(list(module.body), python).items():
        if node in functions:
            functions[node].comprehension_frees = names & frozenset(functions[node].frees)
    for scope in functions.values():
        for name in scope.frees:
            if name not in scope.comprehension_frees:
                scope.owner(name).cells.add(name)
    future_annotations = has_future_annotations(module)
    for scope in functions.values():
        if not future_annotations:
            _check_annotations(scope, filename)
    return ModuleScopes(functions, top_level, classes)


@dataclass
class _Linker:
    """Matches the symbol tables nested in a table with the AST nodes of the
    functions and classes they are for, except in the statements `skip`
    (kept as Python)."""

    functions: dict[FunctionNode, FunctionScope]
    classes: dict[ast.ClassDef, ClassScope]
    skip: Collection[ast.AST]

    def link(
        self,
        table: symtable.SymbolTable,
        body: list[ast.AST],
        parent: FunctionScope | None,
        owner: ClassScope | None,
    ) -> list[FunctionScope]:
        """Link the functions and classes defined in `body`, the body of the
        function `parent` or of the class `owner` (or module code)."""
        tables: dict[tuple[str, int, str], list[symtable.SymbolTable]] = {}
        for child in table.get_children():
            kind = child.get_type()
            if kind in (symtable.SymbolTableType.FUNCTION, symtable.SymbolTableType.CLASS):
                key = (child.get_name(), child.get_lineno(), kind.value)
                tables.setdefault(key, []).append(child)
        scopes = []
        for node in nested_functions(body, self.skip):
            match node:
                case ast.FunctionDef(name=name):
                    pass
                case ast.Lambda():
                    name = "lambda"
                case ast.GeneratorExp():
                    name = "genexpr"
            child = tables[(name, node.lineno, "function")].pop(0)
            assert isinstance(child, symtable.Function)
            scope = _function_scope(child, node, parent, owner)
            self.functions[node] = scope
            scope.children = self.link(child, scope_body(node), scope, None)
            scopes.append(scope)
            if owner is not None:
                owner.methods.append(scope)
        for node in nested_classes(body, self.skip):
            child = tables[(node.name, node.lineno, "class")].pop(0)
            if parent is not None:
                continue  # classes in functions are kept as Python
            qualname = node.name if owner is None else f"{owner.qualname}.{node.name}"
            declared = frozenset(
                s.get_name() for s in child.get_symbols() if s.is_declared_global()
            )
            cls = ClassScope(node.name, qualname, node, owner, declared)
            self.classes[node] = cls
            scopes += self.link(child, list(node.body), None, cls)
        return scopes


def comprehension_captures(
    nodes: list[ast.AST], skip: Collection[ast.AST] = ()
) -> dict[FunctionNode, frozenset[str]]:
    """For every function, lambda and generator expression defined in a
    list, set or dict comprehension: the variables of the comprehensions
    around it (in the same function), which it captures from them rather
    than from the enclosing function."""
    found: dict[FunctionNode, frozenset[str]] = {}

    def visit(node: ast.AST, hidden: frozenset[str]) -> None:
        if node in skip:
            return
        if isinstance(node, FunctionNode):
            if hidden:
                found[node] = hidden
            # Defaults, decorators and the first iterable are evaluated here.
            outer: list[ast.AST] = []
            match node:
                case ast.FunctionDef(args=args, decorator_list=decorators):
                    outer = [*args.defaults, *_present(args.kw_defaults), *decorators]
                case ast.Lambda(args=args):
                    outer = [*args.defaults, *_present(args.kw_defaults)]
                case ast.GeneratorExp(generators=generators):
                    outer = [generators[0].iter]
            for part in outer:
                visit(part, hidden)
            for part in scope_body(node):
                visit(part, frozenset())
            return
        if isinstance(node, ast.ListComp | ast.SetComp | ast.DictComp):
            visit(node.generators[0].iter, hidden)
            inner = hidden | comprehension_variables(node)
            for part in comprehension_parts(node):
                visit(part, inner)
            return
        if isinstance(node, ast.ClassDef):
            for part in [*node.bases, *(k.value for k in node.keywords), *node.decorator_list]:
                visit(part, hidden)
            for part in node.body:
                visit(part, frozenset())
            return
        for child in ast.iter_child_nodes(node):
            visit(child, hidden)

    for node in nodes:
        visit(node, frozenset())
    return found


def _function_scope(
    table: symtable.Function,
    node: FunctionNode,
    parent: FunctionScope | None,
    owner: ClassScope | None = None,
) -> FunctionScope:
    globals_ = set()
    for symbol in table.get_symbols():
        # Includes names declared with a `global` statement.
        if symbol.is_global():
            globals_.add(symbol.get_name())
    match node:
        case ast.FunctionDef(name=name):
            args: ast.arguments | None = node.args
        case ast.Lambda(args=args):
            name = "<lambda>"
        case ast.GeneratorExp():
            name = "<genexpr>"
            args = None
    if parent is not None:
        qualname = f"{parent.qualname}.<locals>.{name}"
    elif owner is not None:
        qualname = f"{owner.qualname}.{name}"
    else:
        qualname = name
    return FunctionScope(
        name=name,
        params=params_of(args),
        # `.0`: the iterator a generator expression receives.
        locals=frozenset(table.get_locals()) - {".0"},
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

CLASS_SCOPE = FunctionScope(name="<class>", params=(), locals=frozenset(), globals=frozenset())
"""Stands for the class body owning the `__class__` cell of its methods."""
