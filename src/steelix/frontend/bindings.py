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

from steelix.frontend.scope import MODULE_SCOPE, FunctionScope

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
    """How the locals of one function are declared in Rust.

    Module level code is analysed the same way with an empty scope, which only
    computes reachability.
    """

    optional: set[str] = field(default_factory=set)
    """Locals read (or deleted) where they might be unassigned: `Option<Bound>`."""
    mutable: set[str] = field(default_factory=set)
    """Locals (and parameters) assigned while possibly already assigned."""
    declared_at: dict[str, ast.stmt] = field(default_factory=dict)
    """Locals declared with `let` at their first assignment (or import) instead
    of at the top."""
    loop_scoped: dict[str, ast.For] = field(default_factory=dict)
    """Loop variables only bound by one `for` and only read inside its body, which
    are declared with `let` at the start of each iteration."""
    assigned_params: set[str] = field(default_factory=set)
    """Parameters that are reassigned, so they are shadowed by an owned `let mut`."""
    declared_at_top: list[str] = field(default_factory=list)
    """Non-parameter locals declared at the top of the function, in source order."""
    assigned_deletes: set[ast.Name] = field(default_factory=set)
    """`del x` targets where `x` is definitely assigned."""
    unreachable: set[ast.stmt] = field(default_factory=set)
    end_reachable: bool = True


@dataclass(frozen=True)
class _Flow:
    definite: frozenset[str]
    """Locals assigned on every path to this point."""
    maybe: frozenset[str]
    """Locals assigned on some path to this point."""


# `None` stands for "unreachable", the identity of `_merge`.
State = _Flow | None


def _merge(*states: State) -> State:
    """The state where control flow from all `states` joins."""
    reachable = [s for s in states if s is not None]
    if not reachable:
        return None
    definite = reachable[0].definite
    maybe = reachable[0].maybe
    for state in reachable[1:]:
        definite &= state.definite
        maybe |= state.maybe
    return _Flow(definite, maybe)


@dataclass
class _Loop:
    breaks: list[State] = field(default_factory=list)
    continues: list[State] = field(default_factory=list)


class _DefiniteAssignment:
    def __init__(self, scope: FunctionScope) -> None:
        self.scope = scope
        self.info = LocalsInfo()
        self.loops: list[_Loop] = []

    def run(self, body: list[ast.stmt]) -> LocalsInfo:
        params = frozenset(self.scope.params)
        for stmt in body:
            for node in ast.walk(stmt):
                if isinstance(node, ast.Delete):
                    # A deleted local can be unassigned again afterwards.
                    for target in node.targets:
                        self.info.optional.update(self._local_names(target))
        end = self.block(body, _Flow(params, params))
        self.info.end_reachable = end is not None
        self._loop_scoped(body)
        self._declarations(body)
        return self.info

    def block(self, stmts: list[ast.stmt], state: State) -> State:
        for stmt in stmts:
            if state is None:
                self.info.unreachable.add(stmt)
                continue
            state = self.stmt(stmt, state)
        return state

    def stmt(self, stmt: ast.stmt, state: _Flow) -> State:
        match stmt:
            case ast.Expr(value):
                self.reads(value, state)
            case ast.Assign(targets, value):
                self.reads(value, state)
                for target in targets:
                    state = self.store(target, state)
            case ast.AugAssign(target, _, value):
                if isinstance(target, ast.Name):
                    self.read_name(target.id, state)
                else:
                    self.reads(target, state)
                self.reads(value, state)
                return self.store(target, state)
            case ast.Delete(targets):
                for target in targets:
                    state = self.delete(target, state)
            case ast.Return(value):
                if value is not None:
                    self.reads(value, state)
                return None
            case ast.If(test, body, orelse):
                self.reads(test, state)
                return _merge(self.block(body, state), self.block(orelse, state))
            case ast.While(test, body, orelse):
                return self.loop(stmt, state, test=test)
            case ast.For(target, iter_, body, orelse):
                self.reads(iter_, state)
                return self.loop(stmt, state, target=target)
            case ast.Break():
                self.loops[-1].breaks.append(state)
                return None
            case ast.Continue():
                self.loops[-1].continues.append(state)
                return None
            case ast.Import(names) | ast.ImportFrom(names=names):
                for alias in names:
                    name = alias.asname or alias.name.split(".")[0]
                    state = self.assign_name(name, state)
            case ast.Assert(test, msg):
                self.reads(test, state)
                if msg is not None:
                    self.reads(msg, state)
            case ast.Pass() | ast.Global() | ast.FunctionDef():
                pass
            case _:
                raise AssertionError(f"unexpected statement {stmt!r}")
        return state

    def loop(
        self,
        stmt: ast.While | ast.For,
        entry: _Flow,
        test: ast.expr | None = None,
        target: ast.expr | None = None,
    ) -> State:
        """`while` and `for`: iterate to a fixed point at the loop head."""
        head = entry
        while True:
            if test is not None:
                self.reads(test, head)
            self.loops.append(_Loop())
            start: State = head if target is None else self.store(target, head)
            end = self.block(stmt.body, start)
            loop = self.loops.pop()
            new_head = _merge(entry, end, *loop.continues)
            assert new_head is not None
            if new_head == head:
                break
            head = new_head
        # The loop ends normally when the iterator is exhausted or the test is
        # false; `while True` only ends through `break` (or `return`).
        exhausted: State = None if test is not None and is_constant_true(test) else head
        return _merge(self.block(stmt.orelse, exhausted), *loop.breaks)

    def store(self, target: ast.expr, state: _Flow) -> _Flow:
        match target:
            case ast.Name(id):
                return self.assign_name(id, state)
            case ast.Tuple(elts) | ast.List(elts):
                for elt in elts:
                    state = self.store(elt, state)
            case ast.Starred(value):
                return self.store(value, state)
            case ast.Attribute(value):
                self.reads(value, state)
            case ast.Subscript(value, index):
                self.reads(value, state)
                self.reads(index, state)
        return state

    def assign_name(self, name: str, state: _Flow) -> _Flow:
        if not self.scope.is_local(name):
            return state
        if name in state.maybe:
            self.info.mutable.add(name)
            if name in self.scope.params:
                self.info.assigned_params.add(name)
        return _Flow(state.definite | {name}, state.maybe | {name})

    def delete(self, target: ast.expr, state: _Flow) -> _Flow:
        match target:
            case ast.Name(id) if self.scope.is_local(id):
                if id in state.definite:
                    self.info.assigned_deletes.add(target)
                return _Flow(state.definite - {id}, state.maybe - {id})
            case ast.Tuple(elts) | ast.List(elts):
                for elt in elts:
                    state = self.delete(elt, state)
            case ast.Attribute(value):
                self.reads(value, state)
            case ast.Subscript(value, index):
                self.reads(value, state)
                self.reads(index, state)
        return state

    def reads(self, expr: ast.expr, state: _Flow) -> None:
        for node in ast.walk(expr):
            if isinstance(node, ast.Name) and isinstance(node.ctx, ast.Load):
                self.read_name(node.id, state)

    def read_name(self, name: str, state: _Flow) -> None:
        if self.scope.is_local(name) and name not in state.definite:
            self.info.optional.add(name)

    def _local_names(self, target: ast.expr) -> list[str]:
        return [
            node.id
            for node in ast.walk(target)
            if isinstance(node, ast.Name)
            and not isinstance(node.ctx, ast.Load)
            and self.scope.is_local(node.id)
        ]

    def _loop_scoped(self, body: list[ast.stmt]) -> None:
        """Find `for` targets that can be declared inside the loop body."""
        bindings: Counter[str] = Counter()
        loads: dict[str, list[ast.Name]] = {}
        loops: list[ast.For] = []
        for stmt in body:
            for node in ast.walk(stmt):
                match node:
                    case ast.Name(name, ast.Load()):
                        loads.setdefault(name, []).append(node)
                    case ast.Name(name):
                        bindings[name] += 1
                    case ast.Import(names) | ast.ImportFrom(names=names):
                        for alias in names:
                            bindings[alias.asname or alias.name.split(".")[0]] += 1
                    case ast.For() if node not in self.info.unreachable:
                        loops.append(node)
        for loop in loops:
            names = _target_names(loop.target)
            if names is None or len(set(names)) != len(names):
                continue
            inside = {id(n) for stmt in loop.body for n in ast.walk(stmt)}
            if all(
                self.scope.is_local(name)
                and name not in self.scope.params
                and name not in self.info.optional
                and bindings[name] == 1
                and all(id(load) in inside for load in loads.get(name, []))
                for name in names
            ):
                for name in names:
                    self.info.loop_scoped[name] = loop

    def _declarations(self, body: list[ast.stmt]) -> None:
        """Pick locals whose first occurrence is a top-level assignment."""
        seen: set[str] = set(self.scope.params) | set(self.info.loop_scoped)
        for stmt in body:
            names = sorted(
                (n for n in ast.walk(stmt) if isinstance(n, ast.Name)),
                key=lambda n: (n.lineno, n.col_offset),
            )
            first = [n.id for n in names]
            for node in ast.walk(stmt):
                if isinstance(node, ast.Import | ast.ImportFrom):
                    first += [alias.asname or alias.name.split(".")[0] for alias in node.names]
            first = [
                name
                for name in dict.fromkeys(first)
                if name not in seen and self.scope.is_local(name)
            ]
            seen.update(first)
            declared = self._declarable(stmt, first)
            for name in first:
                if name in declared:
                    self.info.declared_at[name] = stmt
                else:
                    self.info.declared_at_top.append(name)

    def _declarable(self, stmt: ast.stmt, first: list[str]) -> set[str]:
        """Names a reachable top-level assignment can declare with `let`."""
        if stmt in self.info.unreachable:
            return set()
        if isinstance(stmt, ast.Import | ast.ImportFrom):
            bound = [alias.asname or alias.name.split(".")[0] for alias in stmt.names]
            return {
                name
                for name in bound
                if name in first and bound.count(name) == 1 and name not in self.info.optional
            }
        if not isinstance(stmt, ast.Assign):
            return set()
        if len(stmt.targets) == 1 and isinstance(stmt.targets[0], ast.Tuple | ast.List):
            # A tuple target is declared by a single `let` pattern: all or nothing.
            names = _flat_target_names(stmt.targets[0], starred=True)
            if (
                names is None
                or len(set(names)) != len(names)
                or any(name not in first or name in self.info.optional for name in names)
            ):
                return set()
            return set(names)
        # An `Option` local (one that is deleted later) can be declared here too,
        # as long as the value does not read it first.
        read = {n.id for n in ast.walk(stmt.value) if isinstance(n, ast.Name)}
        return {
            t.id
            for t in stmt.targets
            if isinstance(t, ast.Name) and t.id in first and t.id not in read
        }


def _target_names(target: ast.expr) -> list[str] | None:
    """The names of a target made of names and (nested) tuples, else `None`."""
    match target:
        case ast.Name(id):
            return [id]
        case ast.Tuple(elts) | ast.List(elts):
            names = []
            for elt in elts:
                inner = _target_names(elt)
                if inner is None:
                    return None
                names += inner
            return names
    return None


def _flat_target_names(target: ast.expr, starred: bool = False) -> list[str] | None:
    """`x` or `a, b` (`a, *b` with `starred`): the names, else `None`."""
    match target:
        case ast.Name(id):
            return [id]
        case ast.Tuple(elts) | ast.List(elts):
            names = []
            for elt in elts:
                match elt:
                    case ast.Name(id):
                        names.append(id)
                    case ast.Starred(ast.Name(id)) if starred:
                        names.append(id)
                    case _:
                        return None
            return names
    return None


def is_constant_true(expr: ast.expr) -> bool:
    return isinstance(expr, ast.Constant) and bool(expr.value)


def analyze_locals(fn: ast.FunctionDef, scope: FunctionScope) -> LocalsInfo:
    return _DefiniteAssignment(scope).run(fn.body)


def analyze_module_flow(module: ast.Module) -> LocalsInfo:
    """Reachability of module level statements (there are no locals)."""
    return _DefiniteAssignment(MODULE_SCOPE).run(module.body)
