"""Binding analyses that `symtable` does not provide.

* Module level: how many times each global name is bound, and whether the
  module does anything (`import *`, `globals()`) that makes the set of globals
  unknowable. This decides which calls can be direct Rust calls (design §7).
* Function level: definite-assignment analysis deciding how each local variable
  is declared in Rust (design §5).
"""

from __future__ import annotations

import ast
from collections import Counter
from dataclasses import dataclass, field

from steelix.frontend.scope import FunctionScope

# Names the import system and module creation put into every module's globals.
MODULE_DUNDERS = frozenset(
    {
        "__name__",
        "__doc__",
        "__file__",
        "__spec__",
        "__loader__",
        "__package__",
        "__builtins__",
        "__path__",
        "__cached__",
        "__dict__",
    }
)

# Builtins that expose the calling scope's namespace.
_NAMESPACE_BUILTINS = frozenset({"globals", "vars", "locals"})


# --- module level ----------------------------------------------------------------


@dataclass
class ModuleBindings:
    counts: Counter[str]
    """Number of binding sites of each module global."""
    defs: dict[str, tuple[ast.FunctionDef, int]]
    """Top-level `def` statements by name, with their index in the module body."""
    star_import: bool
    namespace_access: bool
    """The module calls `globals()` / `vars()` / `locals()`."""

    @property
    def dynamic(self) -> bool:
        """Globals can be bound by means invisible to the analysis."""
        return self.star_import or self.namespace_access

    def is_module_bound(self, name: str) -> bool:
        return name in self.counts or name in MODULE_DUNDERS

    def direct_function(self, name: str) -> ast.FunctionDef | None:
        """The `def` that is the only binding of `name`, if calls may bypass globals."""
        if self.dynamic or self.counts[name] != 1 or name not in self.defs:
            return None
        return self.defs[name][0]

    def def_index(self, name: str) -> int:
        return self.defs[name][1]


def analyze_module_bindings(module: ast.Module) -> ModuleBindings:
    counts: Counter[str] = Counter()
    defs: dict[str, tuple[ast.FunctionDef, int]] = {}
    star_import = False

    for index, stmt in enumerate(module.body):
        if isinstance(stmt, ast.FunctionDef):
            defs[stmt.name] = (stmt, index)

    def visit_module_level(node: ast.AST) -> None:
        nonlocal star_import
        for name in _bound_names(node):
            counts[name] += 1
        if isinstance(node, ast.ImportFrom) and any(a.name == "*" for a in node.names):
            star_import = True
        if isinstance(node, ast.FunctionDef | ast.AsyncFunctionDef | ast.ClassDef):
            # The name itself was counted above; the body is another scope.
            declared_global: set[str] = set()
            for child in _scope_children(node):
                visit_nested_scope(child, declared_global)
            return
        if isinstance(
            node, ast.Lambda | ast.ListComp | ast.SetComp | ast.DictComp | ast.GeneratorExp
        ):
            visit_nested_scope(node, declared_global=set())
            return
        for child in ast.iter_child_nodes(node):
            visit_module_level(child)

    def visit_nested_scope(node: ast.AST, declared_global: set[str]) -> None:
        # Inside functions only names declared `global` bind module globals.
        if isinstance(node, ast.Global):
            declared_global.update(node.names)
        if isinstance(node, ast.FunctionDef | ast.AsyncFunctionDef | ast.Lambda | ast.ClassDef):
            inner: set[str] = set()
            if isinstance(node, ast.FunctionDef | ast.AsyncFunctionDef | ast.ClassDef):
                for name in _bound_names(node):
                    if name in declared_global:
                        counts[name] += 1
            for child in _scope_children(node):
                visit_nested_scope(child, inner)
            return
        for name in _bound_names(node):
            if name in declared_global:
                counts[name] += 1
        for child in ast.iter_child_nodes(node):
            visit_nested_scope(child, declared_global)

    for stmt in module.body:
        visit_module_level(stmt)

    namespace_access = _uses_namespace_builtins(module, set())

    return ModuleBindings(counts, defs, star_import, namespace_access)


def _uses_namespace_builtins(node: ast.AST, local_names: set[str]) -> bool:
    """Whether `globals` / `vars` / `locals` is read where it can be the builtin."""
    if isinstance(node, ast.FunctionDef | ast.AsyncFunctionDef | ast.Lambda):
        args = node.args
        inner = {a.arg for a in args.posonlyargs + args.args + args.kwonlyargs}
        inner.update(a.arg for a in (args.vararg, args.kwarg) if a is not None)
        body = node.body if isinstance(node.body, list) else [node.body]
        for stmt in body:
            for child in ast.walk(stmt):
                if isinstance(child, ast.Name) and isinstance(child.ctx, ast.Store):
                    inner.add(child.id)
        # Decorators, defaults and annotations are evaluated in the outer scope.
        outer = [n for n in ast.iter_child_nodes(node) if n not in body and n is not args]
        outer += args.defaults + [d for d in args.kw_defaults if d is not None]
        return any(_uses_namespace_builtins(n, local_names) for n in outer) or any(
            _uses_namespace_builtins(stmt, inner) for stmt in body
        )
    if (
        isinstance(node, ast.Name)
        and isinstance(node.ctx, ast.Load)
        and node.id in _NAMESPACE_BUILTINS
        and node.id not in local_names
    ):
        return True
    return any(_uses_namespace_builtins(c, local_names) for c in ast.iter_child_nodes(node))


def _scope_children(node: ast.AST) -> list[ast.AST]:
    """Children of a scope-introducing node that are evaluated inside the new scope."""
    match node:
        case ast.FunctionDef() | ast.AsyncFunctionDef():
            return list(node.body)
        case ast.ClassDef():
            return list(node.body)
        case ast.Lambda():
            return [node.body]
    return list(ast.iter_child_nodes(node))


def _bound_names(node: ast.AST) -> list[str]:
    """Names bound directly by `node` (not by its children)."""
    match node:
        case ast.Name(id, ast.Store() | ast.Del()):
            return [id]
        case ast.FunctionDef(name) | ast.AsyncFunctionDef(name) | ast.ClassDef(name):
            return [name]
        case ast.Import(names) | ast.ImportFrom(names=names):
            return [
                alias.asname or alias.name.split(".")[0] for alias in names if alias.name != "*"
            ]
        case ast.ExceptHandler(name=str(name)):
            return [name]
        case ast.MatchAs(name=str(name)) | ast.MatchStar(name=str(name)):
            return [name]
        case ast.MatchMapping(rest=str(name)):
            return [name]
        case ast.TypeAlias(ast.Name(id)):
            return [id]
    return []


# --- function level ----------------------------------------------------------------


@dataclass
class LocalsInfo:
    """How the locals of one function are declared in Rust."""

    optional: set[str] = field(default_factory=set)
    """Locals read where they might be unassigned: `Option<Bound>`."""
    mutable: set[str] = field(default_factory=set)
    """Locals (and parameters) assigned while possibly already assigned."""
    declared_at: dict[str, ast.Assign] = field(default_factory=dict)
    """Locals declared with `let` at their first assignment instead of at the top."""
    assigned_params: set[str] = field(default_factory=set)
    """Parameters that are reassigned, so they are shadowed by an owned `let mut`."""
    declared_at_top: list[str] = field(default_factory=list)
    """Non-parameter locals declared at the top of the function, in source order."""
    unreachable: set[ast.stmt] = field(default_factory=set)
    end_reachable: bool = True


# `None` stands for "unreachable": the identity for intersecting definitely
# assigned sets, and for joining maybe-assigned sets.
State = frozenset[str] | None


def _meet(a: State, b: State) -> State:
    if a is None:
        return b
    if b is None:
        return a
    return a & b


def _join(a: State, b: State) -> State:
    if a is None:
        return b
    if b is None:
        return a
    return a | b


class _DefiniteAssignment:
    def __init__(self, scope: FunctionScope) -> None:
        self.scope = scope
        self.info = LocalsInfo()

    def run(self, fn: ast.FunctionDef) -> LocalsInfo:
        params = frozenset(self.scope.params)
        definite, _ = self.block(fn.body, params, params)
        self.info.end_reachable = definite is not None
        self._declarations(fn)
        return self.info

    def block(self, stmts: list[ast.stmt], definite: State, maybe: State) -> tuple[State, State]:
        for stmt in stmts:
            if definite is None:
                self.info.unreachable.add(stmt)
                continue
            assert maybe is not None
            definite, maybe = self.stmt(stmt, definite, maybe)
        return definite, maybe

    def stmt(self, stmt: ast.stmt, definite: frozenset[str], maybe: frozenset[str]):
        match stmt:
            case ast.Expr(value):
                self.reads(value, definite)
            case ast.Assign(targets, value):
                self.reads(value, definite)
                for target in targets:
                    assert isinstance(target, ast.Name)
                    name = target.id
                    if name in maybe:
                        self.info.mutable.add(name)
                        if name in self.scope.params:
                            self.info.assigned_params.add(name)
                    definite = definite | {name}
                    maybe = maybe | {name}
            case ast.Return(value):
                if value is not None:
                    self.reads(value, definite)
                return None, None
            case ast.If(test, body, orelse):
                self.reads(test, definite)
                d1, m1 = self.block(body, definite, maybe)
                d2, m2 = self.block(orelse, definite, maybe)
                return _meet(d1, d2), _join(m1, m2)
            case ast.While(test, body):
                self.reads(test, definite)
                head = maybe
                while True:
                    _, after = self.block(body, definite, head)
                    new_head = _join(head, after)
                    if new_head == head:
                        break
                    head = new_head
                if is_constant_true(test):
                    # No `break` yet, so only `return` leaves `while True`.
                    return None, None
                return definite, head
            case ast.Pass():
                pass
            case _:
                raise AssertionError(f"unexpected statement {stmt!r}")
        return definite, maybe

    def reads(self, expr: ast.expr, definite: frozenset[str]) -> None:
        for node in ast.walk(expr):
            if (
                isinstance(node, ast.Name)
                and isinstance(node.ctx, ast.Load)
                and self.scope.is_local(node.id)
                and node.id not in definite
            ):
                self.info.optional.add(node.id)

    def _declarations(self, fn: ast.FunctionDef) -> None:
        """Pick locals whose first occurrence is a top-level assignment."""
        seen: set[str] = set(self.scope.params)
        for stmt in fn.body:
            names = sorted(
                (n for n in ast.walk(stmt) if isinstance(n, ast.Name)),
                key=lambda n: (n.lineno, n.col_offset),
            )
            for node in names:
                name = node.id
                if name in seen or not self.scope.is_local(name):
                    continue
                seen.add(name)
                if (
                    isinstance(stmt, ast.Assign)
                    and isinstance(node.ctx, ast.Store)
                    and name not in self.info.optional
                    and stmt not in self.info.unreachable
                ):
                    self.info.declared_at[name] = stmt
                else:
                    self.info.declared_at_top.append(name)


def is_constant_true(expr: ast.expr) -> bool:
    return isinstance(expr, ast.Constant) and bool(expr.value)


def analyze_locals(fn: ast.FunctionDef, scope: FunctionScope) -> LocalsInfo:
    return _DefiniteAssignment(scope).run(fn)
